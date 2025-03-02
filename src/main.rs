use bevy::{prelude::*, window::WindowMode};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use shieldtank::prelude::*;

const WINDOW_RESOLUTION: Vec2 = Vec2::new(1280.0, 960.0);

const PROJECT_FILE: &str = "ldtk/dungeon_of_madness.ldtk";

const SKELETON_IID: Iid = iid!("4be48e10-e920-11ef-b902-6dc2806b1269");

const PLAYER_MOVE_SPEED: f32 = 40.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum GameState {
    //#[default]
    //Title,
    #[default]
    Playing,
    //GameOver,
}

#[derive(Component, Reflect)]
struct PlayerMove {
    target: Vec2,
}

#[derive(Event)]
enum PlayerMoveEvent {
    Up,
    Right,
    Down,
    Left,
}

impl PlayerMoveEvent {
    fn as_vec2(&self) -> Vec2 {
        match self {
            PlayerMoveEvent::Up => (0.0, 1.0).into(),
            PlayerMoveEvent::Right => (1.0, 0.0).into(),
            PlayerMoveEvent::Down => (0.0, -1.0).into(),
            PlayerMoveEvent::Left => (-1.0, 0.0).into(),
        }
    }
}

fn main() {
    let log_plugin_settings = bevy::log::LogPlugin {
        level: bevy::log::Level::WARN,
        filter: "wgpu_hal=off,\
            winit=off,\
            bevy_winit=off,\
            bevy_ldtk_asset=debug,\
            shieldtank=debug,\
            dungeon_of_madness=debug"
            .into(),
        ..default()
    };

    let window_plugin_settings: WindowPlugin = WindowPlugin {
        primary_window: Some(Window {
            mode: WindowMode::Windowed,
            resolution: WINDOW_RESOLUTION.into(),
            resizable: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let image_plugin_settings = ImagePlugin::default_nearest();

    let asset_plugin_settings = AssetPlugin {
        meta_check: bevy::asset::AssetMetaCheck::Never,
        ..Default::default()
    };

    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins
            .set(log_plugin_settings)
            .set(window_plugin_settings)
            .set(image_plugin_settings)
            .set(asset_plugin_settings),
        ShieldtankPlugins,
        WorldInspectorPlugin::default(),
    ));

    app.init_state::<GameState>();

    app.add_observer(player_face);
    app.add_observer(player_move_event);

    app.add_systems(Startup, setup);

    app.add_systems(Update, (camera_follow_skeleton, player_keyboard_commands));

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Transform::default().with_scale(Vec2::splat(0.4).extend(1.0)),
    ));

    commands.spawn((
        LevelComponent {
            handle: asset_server.load(PROJECT_FILE.to_string() + "#worlds:Dungeon/Level_0"),
            config: asset_server.add(ProjectConfig {
                levels_override_transform: false,
                ..Default::default()
            }),
        },
        Transform::default(),
    ));
}

#[allow(clippy::type_complexity)]
fn camera_follow_skeleton(
    mut commands: Commands,
    shieldtank_query: ShieldtankQuery,
    camera_query: Query<(Entity, &Transform), (With<Camera2d>, Without<EntityComponent>)>,
) {
    let Some(skeleton) = shieldtank_query.entity_by_iid(SKELETON_IID) else {
        return;
    };

    let (camera_entity, camera_transform) = camera_query.single();

    commands.entity(camera_entity).insert(
        camera_transform.with_translation(
            skeleton
                .global_location()
                .extend(camera_transform.translation.z),
        ),
    );
}

fn player_keyboard_commands(keyboard_input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    let up_pressed = keyboard_input.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
    let right_pressed = keyboard_input.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    let down_pressed = keyboard_input.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);
    let left_pressed = keyboard_input.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);

    match (up_pressed, right_pressed, down_pressed, left_pressed) {
        (true, false, false, false) => commands.trigger(PlayerMoveEvent::Up),
        (false, true, false, false) => commands.trigger(PlayerMoveEvent::Right),
        (false, false, true, false) => commands.trigger(PlayerMoveEvent::Down),
        (false, false, false, true) => commands.trigger(PlayerMoveEvent::Left),
        _ => (),
    };
}
fn player_face(
    trigger: Trigger<PlayerMoveEvent>,
    mut shieldtank_commands: ShieldtankCommands,
    shieldtank_query: ShieldtankQuery,
) {
    let event = trigger.event();

    let Some(skeleton) = shieldtank_query.entity_by_iid(SKELETON_IID) else {
        return;
    };

    match event {
        PlayerMoveEvent::Right => {
            shieldtank_commands.entity(&skeleton).flip_x(false);
        }
        PlayerMoveEvent::Left => {
            shieldtank_commands.entity(&skeleton).flip_x(true);
        }
        _ => {}
    };
}

fn player_move_event(
    trigger: Trigger<PlayerMoveEvent>,
    time: Res<Time>,
    mut shieldtank_commands: ShieldtankCommands,
    shieldtank_query: ShieldtankQuery,
) {
    let event = trigger.event();

    let Some(skeleton) = shieldtank_query.entity_by_iid(SKELETON_IID) else {
        return;
    };

    let skeleton_layer_location = skeleton.level_location();

    let Some(level) = skeleton.get_level() else {
        error!("Skeleton not in a level?");
        return;
    };

    let Some(walls_layer) = level.layer_by_identifier("IntGrid") else {
        error!("No 'IntGrid' layer in level?");
        return;
    };

    let move_attempt =
        skeleton_layer_location + (event.as_vec2() * PLAYER_MOVE_SPEED * time.delta_secs());

    let left_point = move_attempt + Vec2::new(-8.0, 4.0);
    let right_point = move_attempt + Vec2::new(8.0, 4.0);

    let is_touching_wall = walls_layer.int_grid_at(left_point).is_some()
        | walls_layer.int_grid_at(right_point).is_some();

    if is_touching_wall {
        return;
    }

    let is_in_level_bounds = level.location_in_region(move_attempt);

    if !is_in_level_bounds {
        warn!("Out of Bounds!");
        // TODO: switch levels
        return;
    }

    shieldtank_commands
        .entity(&skeleton)
        .set_location(move_attempt);
}
