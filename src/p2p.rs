use bevy::prelude::*;
use matchbox_socket::WebRtcSocketBuilder;

#[derive(Resource, Debug, Clone)]
pub struct MatchboxConfig {
    pub signaling_server: Option<String>,
    pub room: Option<String>,
    pub player_count: usize,
}

impl Default for MatchboxConfig {
    fn default() -> Self {
        Self {
            signaling_server: None,
            room: None,
            player_count: 2,
        }
    }
}

#[derive(Default)]
pub struct RollbackNetworkingPlugin;

impl Plugin for RollbackNetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchboxConfig>();
        app.init_resource::<SocketBuilderResource>();
        app.add_systems(Startup, configure_socket_builder);

        tracing::info!(target: "kingshot::p2p", "Rollback networking plugin initialised");
    }
}

#[derive(Resource, Default)]
pub struct SocketBuilderResource(pub Option<WebRtcSocketBuilder>);

fn configure_socket_builder(
    mut builder: ResMut<SocketBuilderResource>,
    config: Res<MatchboxConfig>,
) {
    let signaling = config
        .signaling_server
        .clone()
        .unwrap_or_else(|| "ws://127.0.0.1:3536/matchbox/".to_owned());

    let room = config
        .room
        .clone()
        .unwrap_or_else(|| "quickstart".to_owned());

    let desired_players = config.player_count;
    let room_url = format!("{signaling}{room}");

    let builder_instance = WebRtcSocketBuilder::new(room_url.clone())
        .add_reliable_channel()
        .add_unreliable_channel();

    builder.0 = Some(builder_instance);

    tracing::info!(
        target: "kingshot::p2p",
        room = room.as_str(),
        room_url,
        desired_players,
        "Matchbox socket builder configured",
    );
}
