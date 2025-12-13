use core::f32;

use bevy::prelude::*;
use bevy::window::WindowMode;
use shieldtank::prelude::*;
use tinyrand::{Rand as _, StdRand};

const WINDOW_RESOLUTION: UVec2 = UVec2::new(1280, 960);
const PROJECT_FILE: &str = "ldtk/dungeon_of_madness.ldtk";
const SKELETON_IID: Iid = iid!("4be48e10-e920-11ef-b902-6dc2806b1269");
const PLAYER_MOVE_SPEED: f32 = 90.0;
const LEVEL_SIZE: f32 = 144.0;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Transform::default().with_scale(Vec2::splat(0.4).extend(1.0)),
    ));

    commands.spawn((
        ShieldtankLevel {
            handle: asset_server.load(format!("{PROJECT_FILE}#world:Dungeon/Start_Hall")),
            ..Default::default()
        },
        Transform::default(),
    ));
}

#[allow(clippy::type_complexity)]
fn camera_follow_skeleton(
    skeleton_query: Query<
        (&Transform, &ShieldtankIid),
        (With<ShieldtankEntity>, Without<Camera2d>),
    >,
    mut camera_transform: Single<&mut Transform, With<Camera2d>>,
) {
    let Some((skeleton_transform, _)) = skeleton_query
        .iter()
        .find(|(_, &ldtk_iid)| *ldtk_iid == SKELETON_IID)
    else {
        return;
    };

    let camera_z = camera_transform.translation.z;
    camera_transform.translation = skeleton_transform.translation.with_z(camera_z);
}

fn player_keyboard_commands(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    grid_query: GridValueQuery,
    level_query: Query<&ShieldtankGlobalBounds, With<ShieldtankLevel>>,
    mut skeleton_query: Query<
        (
            &ShieldtankGlobalBounds,
            &mut ShieldtankTile,
            ShieldtankLocationMut,
            &ShieldtankIid,
        ),
        With<ShieldtankEntity>,
    >,
) {
    let Some((global_bounds, mut tile, mut location, _)) = skeleton_query
        .iter_mut()
        .find(|&(_, _, _, ldtk_iid)| *ldtk_iid == SKELETON_IID)
    else {
        return;
    };

    if !level_query
        .iter()
        .any(|global_bounds| global_bounds.contains(location.get()))
    {
        return;
    }

    let up_pressed = keyboard_input.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
    let right_pressed = keyboard_input.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    let down_pressed = keyboard_input.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);
    let left_pressed = keyboard_input.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);

    const UP: (bool, bool, bool, bool) = (true, false, false, false);
    const UP_RIGHT: (bool, bool, bool, bool) = (true, true, false, false);
    const RIGHT: (bool, bool, bool, bool) = (false, true, false, false);
    const DOWN_RIGHT: (bool, bool, bool, bool) = (false, true, true, false);
    const DOWN: (bool, bool, bool, bool) = (false, false, true, false);
    const DOWN_LEFT: (bool, bool, bool, bool) = (false, false, true, true);
    const LEFT: (bool, bool, bool, bool) = (false, false, false, true);
    const UP_LEFT: (bool, bool, bool, bool) = (true, false, false, true);

    let dir = (up_pressed, right_pressed, down_pressed, left_pressed);

    // Do we need to flip the sprite?
    match dir {
        UP_RIGHT | RIGHT | DOWN_RIGHT => tile.flip_x(false),
        UP_LEFT | LEFT | DOWN_LEFT => tile.flip_x(true),
        _ => {}
    };

    // Construct a direction vector
    let dir = match dir {
        UP => Vec2::new(0.0, 1.0),
        UP_RIGHT => Vec2::new(f32::consts::FRAC_1_SQRT_2, f32::consts::FRAC_1_SQRT_2),
        RIGHT => Vec2::new(1.0, 0.0),
        DOWN_RIGHT => Vec2::new(f32::consts::FRAC_1_SQRT_2, -f32::consts::FRAC_1_SQRT_2),
        DOWN => Vec2::new(0.0, -1.0),
        DOWN_LEFT => Vec2::new(-f32::consts::FRAC_1_SQRT_2, -f32::consts::FRAC_1_SQRT_2),
        LEFT => Vec2::new(-1.0, 0.0),
        UP_LEFT => Vec2::new(-f32::consts::FRAC_1_SQRT_2, f32::consts::FRAC_1_SQRT_2),
        _ => return,
    };

    let new_location = location.get() + dir * time.delta_secs() * PLAYER_MOVE_SPEED;

    let rect = global_bounds.bounds();
    let half_size = rect.half_size();
    let half_width = half_size.x;
    let half_height = half_size.y;

    let sensor1 = Vec2::new(-half_width, half_height);
    let sensor1 = grid_query.grid_value_at(new_location + sensor1).is_none();

    let sensor2 = Vec2::new(half_width, half_height);
    let sensor2 = grid_query.grid_value_at(new_location + sensor2).is_none();

    if sensor1 && sensor2 {
        location.set(new_location);
    }
}

#[allow(clippy::type_complexity)]
fn level_spawn_system(
    // level_query: LdtkLevelQuery<&Name>,
    level_query: Query<(&Name, &ShieldtankGlobalBounds), With<ShieldtankLevel>>,
    // skeleton_query: LdtkEntityQuery<LdtkLocation, Changed<GlobalTransform>>,
    skeleton_query: Query<
        (ShieldtankLocation, &ShieldtankIid),
        (With<ShieldtankEntity>, Changed<GlobalTransform>),
    >,
    asset_server: Res<AssetServer>,
    mut rand: Local<StdRand>,
    mut commands: Commands,
) {
    let Some((skeleton_location, _)) = skeleton_query
        .iter()
        .find(|&(_, ldtk_iid)| *ldtk_iid == SKELETON_IID)
    else {
        return;
    };

    let skeleton_location = skeleton_location.get();

    if level_query
        .iter()
        .any(|(_, global_bounds)| global_bounds.contains(skeleton_location))
    {
        return;
    }

    let level_corner = skeleton_location / LEVEL_SIZE;
    let level_corner = level_corner.floor().as_ivec2();
    let level_grid = level_corner - IVec2::new(0, -1);

    let north_grid = level_grid + IVec2::new(0, 1);
    let east_grid = level_grid + IVec2::new(1, 0);
    let south_grid = level_grid + IVec2::new(0, -1);
    let west_grid = level_grid + IVec2::new(-1, 0);

    let center_offset: Vec2 = Vec2::new(LEVEL_SIZE, -LEVEL_SIZE) / 2.0;

    let level_code_at = |grid: IVec2| -> Option<usize> {
        let center = (grid.as_vec2() * LEVEL_SIZE) + center_offset;
        let level_at = level_query
            .iter()
            .find(|(_, global_bounds)| global_bounds.contains(center))
            .map(|(name, _)| name);
        match &level_at {
            Some(name) if name.as_str() == "Start_Hall" => Some(0),
            Some(name) => name[6..].parse::<usize>().ok(),
            None => None,
        }
    };

    let north_code = level_code_at(north_grid);
    let east_code = level_code_at(east_grid);
    let south_code = level_code_at(south_grid);
    let west_code = level_code_at(west_grid);

    let mut rand = rand.next_lim_usize(15);

    const NORTH_WALL: usize = 0x1;
    const EAST_WALL: usize = 0x2;
    const SOUTH_WALL: usize = 0x4;
    const WEST_WALL: usize = 0x8;

    let mut fix_rand_by_code = |code: Option<usize>, wall: usize, opposite_wall: usize| {
        if let Some(code) = code {
            let wall_at = code & opposite_wall;
            if wall_at == 0 {
                rand &= wall ^ 0xF;
            } else {
                rand |= wall;
            }
        }
    };

    fix_rand_by_code(north_code, NORTH_WALL, SOUTH_WALL);
    fix_rand_by_code(east_code, EAST_WALL, WEST_WALL);
    fix_rand_by_code(west_code, WEST_WALL, EAST_WALL);
    fix_rand_by_code(south_code, SOUTH_WALL, NORTH_WALL);

    let new_level_asset_label = format!("{PROJECT_FILE}#world:Dungeon/Level_{rand}");
    let spawn_corner = level_grid.as_vec2() * LEVEL_SIZE;

    commands.spawn((
        ShieldtankLevel {
            handle: asset_server.load(new_level_asset_label),
            ..Default::default()
        },
        Transform::default().with_translation(spawn_corner.extend(0.0)),
    ));
}

fn main() {
    let log_plugin_settings = bevy::log::LogPlugin {
        level: bevy::log::Level::WARN,
        filter: "wgpu_hal=off,\
            winit=off,\
            bevy_winit=off,\
            bevy_ldtk_asset=info,\
            shieldtank=info,\
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
    ));

    #[cfg(debug_assertions)]
    {
        use bevy_inspector_egui::bevy_egui::EguiPlugin;
        use bevy_inspector_egui::quick::WorldInspectorPlugin;
        app.add_plugins(EguiPlugin::default())
            .add_plugins(WorldInspectorPlugin::default());
    }

    app.add_systems(Startup, setup);

    app.add_systems(
        Update,
        (
            camera_follow_skeleton,
            player_keyboard_commands,
            level_spawn_system,
        ),
    );

    app.run();
}
