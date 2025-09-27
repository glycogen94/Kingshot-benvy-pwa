use bevy::prelude::*;
use bevy::prelude::{AppExtStates, NextState, OnEnter, States};

/// Registers globally shared resources, application state, and lifecycle glue
/// so higher-level gameplay plugins can remain focused on their domains.
pub struct KingshotCorePlugin;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,
    Gameplay,
    Pause,
    GameOver,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum SessionStatus {
    #[default]
    Idle,
    Loading,
    InMenu,
    Running,
    Paused,
    Complete,
}

#[derive(Resource, Debug, Clone)]
pub struct KingshotConfig {
    pub base_aspect_ratio: f32,
    pub target_fps: f32,
}

impl Default for KingshotConfig {
    fn default() -> Self {
        Self {
            base_aspect_ratio: 9.0 / 16.0,
            target_fps: 60.0,
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct GameSession {
    pub wave: u32,
    pub score: u32,
    pub status: SessionStatus,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct AppStateRequest {
    pub next: AppState,
}

impl AppStateRequest {
    pub fn new(next: AppState) -> Self {
        Self { next }
    }
}

impl Plugin for KingshotCorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(KingshotConfig::default());
        app.insert_resource(GameSession::default());
        app.add_event::<AppStateRequest>();
        app.init_state::<AppState>();
        app.add_systems(PreUpdate, apply_state_requests);
        app.add_systems(Startup, log_startup_config);
        app.add_systems(OnEnter(AppState::Loading), mark_loading_begin);
        app.add_systems(OnEnter(AppState::MainMenu), mark_menu_begin);
        app.add_systems(OnEnter(AppState::Gameplay), mark_gameplay_begin);
        app.add_systems(OnEnter(AppState::Pause), mark_pause_begin);
        app.add_systems(OnEnter(AppState::GameOver), mark_game_over);
    }
}

fn apply_state_requests(
    mut requests: EventReader<AppStateRequest>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if let Some(request) = requests.read().last().copied() {
        next_state.set(request.next);
    }
}

fn mark_loading_begin(mut session: ResMut<GameSession>) {
    session.status = SessionStatus::Loading;
}

fn mark_menu_begin(mut session: ResMut<GameSession>) {
    session.status = SessionStatus::InMenu;
}

fn mark_gameplay_begin(mut session: ResMut<GameSession>) {
    session.status = SessionStatus::Running;
}

fn mark_pause_begin(mut session: ResMut<GameSession>) {
    session.status = SessionStatus::Paused;
}

fn mark_game_over(mut session: ResMut<GameSession>) {
    session.status = SessionStatus::Complete;
}

fn log_startup_config(config: Res<KingshotConfig>) {
    tracing::info!(
        "Kingshot config initialised: aspect_ratio={:.3}, target_fps={}",
        config.base_aspect_ratio,
        config.target_fps
    );
}
