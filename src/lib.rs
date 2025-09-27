mod core;
mod game;
mod input;
mod logging;
mod offscreen;
#[cfg(feature = "p2p")]
mod p2p;

use crate::core::KingshotCorePlugin;
use crate::game::KingshotGamePlugin;
use crate::input::{KingshotInputPlugin, PointerEventPayload, push_pointer_event};
use bevy::{
    app::PluginsState,
    prelude::*,
    render::{
        RenderPlugin,
        camera::{CameraProjection, Projection},
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::{ExitCondition, PresentMode, WindowResolution},
    winit::WinitPlugin,
};
use offscreen::{OffscreenCanvasResource, insert_offscreen_handles};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;
use web_sys::OffscreenCanvas;

#[derive(Resource, Copy, Clone, Debug)]
struct CanvasSize {
    logical_width: f32,
    logical_height: f32,
    scale_factor: f32,
}

impl CanvasSize {
    fn new(physical_width: f32, physical_height: f32, scale_factor: f32) -> Self {
        let safe_scale = if scale_factor <= 0.0 {
            1.0
        } else {
            scale_factor
        };
        let logical_width = physical_width / safe_scale;
        let logical_height = physical_height / safe_scale;

        Self {
            logical_width,
            logical_height,
            scale_factor: safe_scale,
        }
    }
}

fn apply_initial_canvas_size(mut windows: Query<&mut Window>, canvas: Res<CanvasSize>) {
    if let Ok(mut window) = windows.single_mut() {
        window
            .resolution
            .set(canvas.logical_width, canvas.logical_height);
        window.resolution.set_scale_factor(canvas.scale_factor);
    }
}

fn update_camera_projection(mut query: Query<&mut Projection>, canvas: Res<CanvasSize>) {
    for mut projection in &mut query {
        projection.update(canvas.logical_width, canvas.logical_height);
    }
}

#[wasm_bindgen]
pub struct KingshotApp {
    app: App,
}

#[wasm_bindgen]
impl KingshotApp {
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: OffscreenCanvas,
        physical_width: f32,
        physical_height: f32,
        scale_factor: f32,
    ) -> Result<KingshotApp, JsValue> {
        logging::init();

        let canvas_size = CanvasSize::new(physical_width, physical_height, scale_factor);

        let mut app = App::new();

        app.insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.04)));
        app.insert_resource(canvas_size);
        app.insert_non_send_resource(OffscreenCanvasResource::new(canvas));

        let window_plugin = WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(
                    canvas_size.logical_width,
                    canvas_size.logical_height,
                )
                .with_scale_factor_override(canvas_size.scale_factor),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                visible: true,
                ..Default::default()
            }),
            exit_condition: ExitCondition::DontExit,
            ..Default::default()
        };

        app.add_plugins(
            DefaultPlugins
                .set(window_plugin)
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::BROWSER_WEBGPU),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(AssetPlugin {
                    watch_for_changes_override: Some(false),
                    ..Default::default()
                })
                .disable::<WinitPlugin>(),
        );

        app.add_systems(PreStartup, insert_offscreen_handles);
        app.add_systems(Startup, apply_initial_canvas_size);
        app.add_systems(Update, update_camera_projection);
        app.add_plugins((KingshotCorePlugin, KingshotInputPlugin, KingshotGamePlugin));

        #[cfg(feature = "p2p")]
        app.add_plugins(p2p::RollbackNetworkingPlugin::default());

        Ok(KingshotApp { app })
    }

    #[wasm_bindgen]
    pub fn update(&mut self) {
        match self.app.plugins_state() {
            PluginsState::Ready => {
                self.app.finish();
                self.app.cleanup();
            }
            PluginsState::Finished => {
                self.app.cleanup();
            }
            PluginsState::Cleaned => {
                self.app.update();
            }
            PluginsState::Adding => {}
        }
    }

    #[wasm_bindgen]
    pub fn resize(&mut self, physical_width: f32, physical_height: f32, scale_factor: f32) {
        let canvas_size = CanvasSize::new(physical_width, physical_height, scale_factor);

        {
            let world = self.app.world_mut();
            if let Some(mut size_res) = world.get_resource_mut::<CanvasSize>() {
                *size_res = canvas_size;
            } else {
                world.insert_resource(canvas_size);
            }

            let mut windows = world.query::<&mut Window>();
            for mut window in windows.iter_mut(world) {
                window
                    .resolution
                    .set(canvas_size.logical_width, canvas_size.logical_height);
                window.resolution.set_scale_factor(canvas_size.scale_factor);
            }

            let mut projections = world.query::<&mut Projection>();
            for mut projection in projections.iter_mut(world) {
                projection.update(canvas_size.logical_width, canvas_size.logical_height);
            }
        }
    }

    #[wasm_bindgen]
    pub fn handle_pointer_event(&mut self, value: JsValue) -> Result<(), JsValue> {
        let payload: PointerEventPayload = from_value(value).map_err(|error| {
            JsValue::from_str(&format!("failed to decode pointer event: {error}"))
        })?;

        {
            let world = self.app.world_mut();
            push_pointer_event(world, payload);
        }

        Ok(())
    }
}
