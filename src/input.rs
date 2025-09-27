use bevy::{math::Vec2, prelude::*};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PointerPhase {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(default)]
pub struct PointerModifiers {
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointerEventPayload {
    pub phase: PointerPhase,
    pub pointer_id: i32,
    pub pointer_type: String,
    pub buttons: u32,
    pub client_x: f32,
    pub client_y: f32,
    pub canvas_x: f32,
    pub canvas_y: f32,
    pub normalized_x: f32,
    pub normalized_y: f32,
    pub pressure: f32,
    pub timestamp: f64,
    #[serde(default)]
    pub modifiers: PointerModifiers,
    #[serde(default)]
    pub delta_x: f32,
    #[serde(default)]
    pub delta_y: f32,
    #[serde(default)]
    pub wheel_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerDevice {
    Touch,
    Mouse,
    Pen,
    Unknown,
}

impl PointerDevice {
    fn from_str(pointer_type: &str) -> Self {
        match pointer_type.to_ascii_lowercase().as_str() {
            "touch" => Self::Touch,
            "mouse" => Self::Mouse,
            "pen" => Self::Pen,
            _ => Self::Unknown,
        }
    }
}

#[derive(Event, Debug, Clone)]
#[allow(dead_code)]
pub struct PointerInputEvent {
    pub phase: PointerPhase,
    pub pointer_id: i32,
    pub device: PointerDevice,
    pub buttons: u32,
    pub canvas_position: Vec2,
    pub normalized_position: Vec2,
    pub client_position: Vec2,
    pub delta: Vec2,
    pub pressure: f32,
    pub wheel_delta: f32,
    pub timestamp: f64,
    pub modifiers: PointerModifiers,
}

impl PointerInputEvent {
    fn from_payload(payload: PointerEventPayload) -> Self {
        let device = PointerDevice::from_str(&payload.pointer_type);
        Self {
            phase: payload.phase,
            pointer_id: payload.pointer_id,
            device,
            buttons: payload.buttons,
            canvas_position: Vec2::new(payload.canvas_x, payload.canvas_y),
            normalized_position: Vec2::new(payload.normalized_x, payload.normalized_y),
            client_position: Vec2::new(payload.client_x, payload.client_y),
            delta: Vec2::new(payload.delta_x, payload.delta_y),
            pressure: payload.pressure,
            wheel_delta: payload.wheel_delta,
            timestamp: payload.timestamp,
            modifiers: payload.modifiers,
        }
    }
}

pub struct KingshotInputPlugin;

impl Plugin for KingshotInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PointerInputEvent>();
    }
}

pub fn push_pointer_event(world: &mut World, payload: PointerEventPayload) {
    let event = PointerInputEvent::from_payload(payload);
    world.send_event(event);
}
