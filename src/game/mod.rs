use bevy::math::primitives::{Circle, Cuboid, Sphere};
use bevy::prelude::Ray3d;
use bevy::prelude::*;
use bevy::prelude::{OnEnter, OnExit, in_state};
use bevy::render::camera::Camera;
use bevy::time::TimerMode;
use std::collections::HashSet;

use crate::core::{AppState, AppStateRequest, GameSession, SessionStatus};
use crate::input::{PointerInputEvent, PointerPhase};

pub struct KingshotGamePlugin;

impl Plugin for KingshotGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GameplayEvent>();
        app.add_plugins((LoadingFlowPlugin, MainMenuFlowPlugin, GameplayFlowPlugin));
    }
}

#[derive(Event, Debug, Clone, Copy)]
pub enum GameplayEvent {
    LoadingComplete,
    StartCampaign,
}

#[derive(Resource, Default)]
struct AimTarget {
    position: Vec3,
    valid: bool,
}

#[derive(Resource, Clone)]
struct GameplayAssets {
    projectile_mesh: Handle<Mesh>,
    projectile_material: Handle<StandardMaterial>,
}

/// Handles boot flow and staging asset load.
struct LoadingFlowPlugin;

impl Plugin for LoadingFlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Loading), begin_loading);
        app.add_systems(OnExit(AppState::Loading), cleanup_loading);
        app.add_systems(
            Update,
            drive_loading_timer.run_if(in_state(AppState::Loading)),
        );
    }
}

#[derive(Resource)]
struct LoadingTimer(Timer);

fn begin_loading(mut commands: Commands, mut session: ResMut<GameSession>) {
    session.wave = 0;
    session.score = 0;
    commands.insert_resource(LoadingTimer(Timer::from_seconds(1.0, TimerMode::Once)));
}

fn drive_loading_timer(
    time: Res<Time>,
    mut timer: ResMut<LoadingTimer>,
    mut events: EventWriter<GameplayEvent>,
    mut requests: EventWriter<AppStateRequest>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        events.write(GameplayEvent::LoadingComplete);
        requests.write(AppStateRequest::new(AppState::MainMenu));
    }
}

fn cleanup_loading(mut commands: Commands) {
    commands.remove_resource::<LoadingTimer>();
}

/// Temporary main menu placeholder that hands off to gameplay automatically.
struct MainMenuFlowPlugin;

impl Plugin for MainMenuFlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), enter_menu);
        app.add_systems(OnExit(AppState::MainMenu), exit_menu);
        app.add_systems(
            Update,
            drive_menu_auto_start.run_if(in_state(AppState::MainMenu)),
        );
    }
}

#[derive(Component)]
struct MainMenuMarker;

#[derive(Resource)]
struct MenuAutoStartTimer(Timer);

fn enter_menu(mut commands: Commands, mut events: EventWriter<GameplayEvent>) {
    events.write(GameplayEvent::StartCampaign);
    commands.insert_resource(MenuAutoStartTimer(Timer::from_seconds(
        1.5,
        TimerMode::Once,
    )));
    commands.spawn((Name::new("MainMenuAnchor"), MainMenuMarker));
}

fn drive_menu_auto_start(
    time: Res<Time>,
    mut timer: ResMut<MenuAutoStartTimer>,
    mut requests: EventWriter<AppStateRequest>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        requests.write(AppStateRequest::new(AppState::Gameplay));
    }
}

fn exit_menu(mut commands: Commands, query: Query<Entity, With<MainMenuMarker>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<MenuAutoStartTimer>();
}

/// Builds the placeholder combat sandbox used while we wire inputs and real gameplay.
struct GameplayFlowPlugin;

impl Plugin for GameplayFlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Gameplay), setup_gameplay_scene);
        app.add_systems(
            Update,
            (
                process_pointer_inputs,
                update_aim_reticle,
                integrate_projectiles,
                advance_enemies,
                resolve_projectile_enemy_collisions,
            )
                .chain()
                .run_if(in_state(AppState::Gameplay)),
        );
        app.add_systems(OnExit(AppState::Gameplay), teardown_gameplay_scene);
    }
}

#[derive(Component)]
struct GameplayEntity;

#[derive(Component)]
struct HeroCore;

#[derive(Component)]
struct ProjectileAnchor;

#[derive(Component)]
struct AimReticle;

#[derive(Component)]
struct Projectile {
    velocity: Vec3,
    lifetime: Timer,
}

#[derive(Component)]
struct Enemy {
    speed: f32,
}

fn setup_gameplay_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut session: ResMut<GameSession>,
) {
    session.wave = 1;
    session.score = 0;

    let projectile_mesh = meshes.add(Sphere::new(0.35));
    let projectile_material = materials.add(Color::srgb(0.95, 0.6, 0.2));
    let enemy_mesh = meshes.add(Cuboid::new(1.0, 1.6, 1.0));
    let enemy_material = materials.add(Color::srgb(0.25, 0.5, 0.75));
    let reticle_mesh = meshes.add(Circle::new(0.55));
    let reticle_material = materials.add(Color::srgb(0.95, 0.3, 0.4));

    commands.insert_resource(GameplayAssets {
        projectile_mesh: projectile_mesh.clone(),
        projectile_material: projectile_material.clone(),
    });
    commands.insert_resource(AimTarget::default());

    let root = commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Name::new("GameplayRoot"),
            GameplayEntity,
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        // Ground disc representing the castle courtyard.
        parent.spawn((
            Mesh3d(meshes.add(Circle::new(6.5))),
            MeshMaterial3d(materials.add(Color::srgb_u8(45, 50, 60))),
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Name::new("Courtyard"),
        ));

        // Hero placeholder at the center of the field.
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.1, 2.2, 1.1))),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.25, 0.35))),
            Transform::from_xyz(0.0, 1.1, 0.0),
            GlobalTransform::default(),
            HeroCore,
            Name::new("HeroCore"),
        ));

        // Anchor that will later emit projectiles/abilities.
        parent.spawn((
            Transform::from_xyz(0.0, 2.2, 0.0),
            GlobalTransform::default(),
            ProjectileAnchor,
            Name::new("ProjectileAnchor"),
        ));

        parent.spawn((
            Mesh3d(reticle_mesh.clone()),
            MeshMaterial3d(reticle_material.clone()),
            Transform::from_xyz(0.0, 0.05, 0.0).with_scale(Vec3::splat(0.01)),
            GlobalTransform::default(),
            AimReticle,
            Name::new("AimReticle"),
        ));

        for (index, position) in initial_enemy_positions().iter().enumerate() {
            parent.spawn((
                Mesh3d(enemy_mesh.clone()),
                MeshMaterial3d(enemy_material.clone()),
                Transform::from_translation(*position),
                GlobalTransform::default(),
                Enemy {
                    speed: 1.25 + index as f32 * 0.25,
                },
                GameplayEntity,
                Name::new(format!("Enemy#{index}")),
            ));
        }

        parent.spawn((
            PointLight {
                intensity: 3_200.0,
                shadows_enabled: true,
                range: 25.0,
                ..default()
            },
            Transform::from_xyz(5.0, 7.5, 5.0),
            GlobalTransform::default(),
            Name::new("KeyLight"),
        ));

        parent.spawn((
            DirectionalLight {
                shadows_enabled: true,
                illuminance: 18_000.0,
                ..default()
            },
            Transform::from_xyz(-8.0, 12.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y),
            GlobalTransform::default(),
            Name::new("Sun"),
        ));

        parent.spawn((
            Camera3d::default(),
            Transform::from_xyz(-10.0, 8.0, 14.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
            GlobalTransform::default(),
            Name::new("GameplayCamera"),
        ));
    });
}

fn process_pointer_inputs(
    mut commands: Commands,
    mut events: EventReader<PointerInputEvent>,
    mut aim_target: ResMut<AimTarget>,
    assets: Option<Res<GameplayAssets>>,
    mut hero_query: Query<&mut Transform, With<HeroCore>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        aim_target.valid = false;
        return;
    };

    let Some(assets) = assets else {
        aim_target.valid = false;
        return;
    };

    for event in events.read() {
        match event.phase {
            PointerPhase::Cancel | PointerPhase::End => {
                aim_target.valid = false;
                continue;
            }
            _ => {}
        }

        let Some(world_position) =
            pointer_on_ground(camera, camera_transform, event.normalized_position)
        else {
            continue;
        };

        aim_target.position = world_position;
        aim_target.valid = true;

        let mut hero_origin = Vec3::new(0.0, 1.2, 0.0);

        if let Some(mut hero_transform) = hero_query.iter_mut().next() {
            hero_origin = hero_transform.translation + Vec3::Y * 1.2;
            let look_target = Vec3::new(
                world_position.x,
                hero_transform.translation.y,
                world_position.z,
            );
            let forward = (look_target - hero_transform.translation).normalize_or_zero();

            if forward.length_squared() > f32::EPSILON {
                let rotation = Quat::from_rotation_arc(Vec3::Z, forward);
                hero_transform.rotation = rotation;
            }
        }

        if matches!(event.phase, PointerPhase::Start) {
            let direction = (world_position - hero_origin).normalize_or_zero();
            if direction.length_squared() > 0.0001 {
                spawn_projectile(&mut commands, &assets, hero_origin, direction);
            }
        }
    }
}

fn update_aim_reticle(
    aim_target: Res<AimTarget>,
    mut query: Query<&mut Transform, With<AimReticle>>,
) {
    if !aim_target.is_changed() {
        return;
    }

    if let Some(mut transform) = query.iter_mut().next() {
        if aim_target.valid {
            transform.translation = Vec3::new(aim_target.position.x, 0.05, aim_target.position.z);
            transform.scale = Vec3::splat(1.0);
        } else {
            transform.scale = Vec3::splat(0.01);
        }
    }
}

fn integrate_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile)>,
) {
    for (entity, mut transform, mut projectile) in &mut projectiles {
        transform.translation += projectile.velocity * time.delta_secs();
        projectile.lifetime.tick(time.delta());

        if projectile.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn advance_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut enemies: Query<(Entity, &mut Transform, &Enemy)>,
    mut requests: EventWriter<AppStateRequest>,
    session: Res<GameSession>,
) {
    let mut triggered_game_over = false;

    for (entity, mut transform, enemy) in &mut enemies {
        let direction = (Vec3::ZERO - transform.translation).normalize_or_zero();
        transform.translation += direction * enemy.speed * time.delta_secs();

        if transform.translation.length_squared() <= 0.6_f32.powi(2) {
            commands.entity(entity).despawn();

            if !triggered_game_over && session.status != SessionStatus::Complete {
                requests.write(AppStateRequest::new(AppState::GameOver));
                triggered_game_over = true;
            }
        }
    }
}

fn resolve_projectile_enemy_collisions(
    mut commands: Commands,
    projectiles: Query<(Entity, &Transform), With<Projectile>>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    mut session: ResMut<GameSession>,
) {
    let mut enemies_to_remove: HashSet<Entity> = HashSet::default();
    let mut projectiles_to_remove = Vec::new();

    for (projectile_entity, projectile_transform) in &projectiles {
        for (enemy_entity, enemy_transform) in &enemies {
            if enemies_to_remove.contains(&enemy_entity) {
                continue;
            }

            let distance_sq = projectile_transform
                .translation
                .distance_squared(enemy_transform.translation);

            if distance_sq <= 0.9_f32.powi(2) {
                projectiles_to_remove.push(projectile_entity);
                enemies_to_remove.insert(enemy_entity);
                session.score = session.score.saturating_add(100);
                break;
            }
        }
    }

    for entity in projectiles_to_remove {
        commands.entity(entity).despawn();
    }

    for entity in enemies_to_remove {
        commands.entity(entity).despawn();
    }
}

fn pointer_on_ground(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    normalized: Vec2,
) -> Option<Vec3> {
    let viewport_size = camera.logical_viewport_size()?;

    let screen_position = Vec2::new(
        normalized.x * viewport_size.x,
        (1.0 - normalized.y) * viewport_size.y,
    );

    let ray: Ray3d = camera
        .viewport_to_world(camera_transform, screen_position)
        .ok()?;
    if ray.direction.y.abs() <= 0.0001 {
        return None;
    }

    let distance = -ray.origin.y / ray.direction.y;
    if distance <= 0.0 {
        return None;
    }

    Some(ray.origin + ray.direction * distance)
}

fn spawn_projectile(
    commands: &mut Commands,
    assets: &GameplayAssets,
    origin: Vec3,
    direction: Vec3,
) {
    let forward = direction.normalize();
    let velocity = forward * 18.0;

    commands.spawn((
        Mesh3d(assets.projectile_mesh.clone()),
        MeshMaterial3d(assets.projectile_material.clone()),
        Transform::from_translation(origin + forward * 0.4),
        GlobalTransform::default(),
        Projectile {
            velocity,
            lifetime: Timer::from_seconds(1.2, TimerMode::Once),
        },
        GameplayEntity,
        Name::new("Projectile"),
    ));
}

fn initial_enemy_positions() -> [Vec3; 3] {
    [
        Vec3::new(7.5, 0.8, 6.0),
        Vec3::new(-7.0, 0.8, 3.5),
        Vec3::new(0.0, 0.8, -8.0),
    ]
}

fn teardown_gameplay_scene(mut commands: Commands, query: Query<Entity, With<GameplayEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<GameplayAssets>();
    commands.remove_resource::<AimTarget>();
}
