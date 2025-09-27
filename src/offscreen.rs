use std::{ptr::NonNull, thread::ThreadId};

use bevy::{prelude::*, window::Window};
use bevy_window::{RawHandleWrapper, WindowWrapper};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};
use web_sys::OffscreenCanvas;

pub struct OffscreenCanvasResource(OffscreenCanvas);

impl OffscreenCanvasResource {
    pub fn new(canvas: OffscreenCanvas) -> Self {
        Self(canvas)
    }

    pub fn canvas(&self) -> &OffscreenCanvas {
        &self.0
    }
}

pub fn insert_offscreen_handles(
    mut commands: Commands,
    canvas: NonSend<OffscreenCanvasResource>,
    mut new_windows: Query<Entity, Added<Window>>,
) {
    let Some(entity) = new_windows.iter_mut().next() else {
        return;
    };

    let handle = OffscreenWindowHandle::new(canvas.canvas());
    let wrapper = RawHandleWrapper::new(&WindowWrapper::new(handle))
        .expect("Failed to wrap OffscreenCanvas window handle");

    commands.entity(entity).insert(wrapper);
}

struct OffscreenWindowHandle {
    window_handle: RawWindowHandle,
    display_handle: RawDisplayHandle,
    thread_id: ThreadId,
}

impl OffscreenWindowHandle {
    fn new(canvas: &OffscreenCanvas) -> Self {
        // SAFETY: OffscreenCanvas remains alive while Bevy runs because it's stored in a resource.
        let ptr = NonNull::from(canvas).cast();
        let window = raw_window_handle::WebOffscreenCanvasWindowHandle::new(ptr);
        let window_handle = RawWindowHandle::WebOffscreenCanvas(window);
        let display_handle = DisplayHandle::web().as_raw();

        Self {
            window_handle,
            display_handle,
            thread_id: std::thread::current().id(),
        }
    }
}

unsafe impl Send for OffscreenWindowHandle {}
unsafe impl Sync for OffscreenWindowHandle {}

impl HasWindowHandle for OffscreenWindowHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        if self.thread_id != std::thread::current().id() {
            return Err(HandleError::NotSupported);
        }

        // SAFETY: The handle originates from the OffscreenCanvas and is valid for the lifetime of the canvas.
        Ok(unsafe { WindowHandle::borrow_raw(self.window_handle) })
    }
}

impl HasDisplayHandle for OffscreenWindowHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        if self.thread_id != std::thread::current().id() {
            return Err(HandleError::NotSupported);
        }

        Ok(unsafe { DisplayHandle::borrow_raw(self.display_handle) })
    }
}
