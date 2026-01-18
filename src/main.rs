use std::f32::consts::FRAC_1_SQRT_2;

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use bevy::window::WindowMode;
use bevy::{color::palettes::tailwind::GRAY_500, input::mouse::MouseWheel};
use shieldtank::prelude::*;
use tinyrand::{Rand as _, StdRand};

const WINDOW_RESOLUTION: UVec2 = UVec2::new(1280, 960);

const PROJECT_FILE: &str = "ldtk/dungeon_of_madness.ldtk";
const SKELETON_IID: u128 = iid!("4be48e10-e920-11ef-b902-6dc2806b1269").as_u128();
const START_HALL_IID: u128 = iid!("29c72090-1030-11f0-8f0e-c7ebf6f05d5f").as_u128();
const LEVEL_SIZE: f32 = 144.0;

const BACKGROUND_SHADER_PATH: &str = "shaders/background.wgsl";
const BACKGROUND_Z: f32 = -100.0;

const PLAYER_MOVE_SPEED: f32 = 90.0;

const CAMERA_ZOOM_DEFAULT: f32 = 0.4;
const CAMERA_ZOOM_SPEED: f32 = 3.0;
const CAMERA_ZOOM_MIN: f32 = 0.1;
const CAMERA_ZOOM_MAX: f32 = 2.0;

const LEVEL_UP: Vec2 = Vec2::new(0.0, LEVEL_SIZE);
const LEVEL_RIGHT: Vec2 = Vec2::new(LEVEL_SIZE, 0.0);
const LEVEL_DOWN: Vec2 = Vec2::new(0.0, -LEVEL_SIZE);
const LEVEL_LEFT: Vec2 = Vec2::new(-LEVEL_SIZE, 0.0);

const WALL_UP: u16 = 0x1;
const WALL_RIGHT: u16 = 0x2;
const WALL_DOWN: u16 = 0x4;
const WALL_LEFT: u16 = 0x8;

#[derive(Resource, Deref, DerefMut)]
struct CurrentLevel(Entity);

#[derive(Event, Deref)]
struct AttemptSpawnLevel(Vec2);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    Playing,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct BackgroundMaterial {
    #[texture(0)]
    #[sampler(1)]
    color_texture: Option<Handle<Image>>,
}

#[derive(Component)]
struct Background;

impl Material2d for BackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        BACKGROUND_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::sprite_render::AlphaMode2d {
        AlphaMode2d::Mask(0.5)
    }
}

fn parse_level_code(level_name: &str) -> u16 {
    if level_name == "Start_Hall" {
        0
    } else {
        level_name[6..].parse().expect("bad level name?")
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    mut commands: Commands,
) {
    commands.spawn((
        Camera2d,
        Transform::default().with_scale(Vec2::splat(CAMERA_ZOOM_DEFAULT).extend(1.0)),
    ));

    // The start hall, which also contains the player skeleton in the
    // `Entities` layer.
    commands.spawn((
        ShieldtankLevel {
            handle: asset_server.load(format!("{PROJECT_FILE}#world:Dungeon/Start_Hall")),
            ..Default::default()
        },
        Transform::default(),
    ));

    // A text banner at the bottom describing the player keybinds.
    commands.spawn((
        Text::new("Movement: WASD or Arrow Keys\nZoom in/out: Mouse Scroll"),
        TextFont {
            font: asset_server.load("fonts/IMMORTAL.ttf"),
            font_size: 22.0,
            ..Default::default()
        },
        TextColor(GRAY_500.into()),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(40.0),
            left: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        },
    ));

    // The foreground mesh.
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::from_corners(
            Vec2::splat(-999.0),
            Vec2::splat(999.0),
        ))),
        MeshMaterial2d(materials.add(BackgroundMaterial {
            color_texture: Some(asset_server.load("textures/rust.png")),
        })),
        Transform::from_translation(Vec2::ZERO.extend(BACKGROUND_Z)),
        Background,
    ));
}

#[allow(clippy::type_complexity)]
fn camera_follow_skeleton(
    skeleton_transform: SingleByIid<
        SKELETON_IID,
        &Transform,
        (
            With<ShieldtankEntity>,
            Without<Camera2d>,
            Without<Background>,
        ),
    >,
    mut camera_transform: Single<
        &mut Transform,
        (
            Without<ShieldtankEntity>,
            With<Camera2d>,
            Without<Background>,
        ),
    >,
    mut background_transform: Single<
        &mut Transform,
        (
            Without<ShieldtankEntity>,
            Without<Camera2d>,
            With<Background>,
        ),
    >,
) {
    let skeleton_location = skeleton_transform.translation;

    let camera_z = camera_transform.translation.z;

    camera_transform.translation = skeleton_location.with_z(camera_z);
    background_transform.translation = skeleton_location.with_z(BACKGROUND_Z);
}

fn camera_zoom_commands(
    time: Res<Time>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
    mut mouse_scroll: MessageReader<MouseWheel>,
) {
    for scroll_message in mouse_scroll.read() {
        let scroll_amount = scroll_message.y.signum() * time.delta_secs() * CAMERA_ZOOM_SPEED;
        let new_zoom = (camera.scale.x - scroll_amount).clamp(CAMERA_ZOOM_MIN, CAMERA_ZOOM_MAX);
        camera.scale = Vec2::splat(new_zoom).extend(1.0);
    }
}

fn wait_for_start_hall(
    level_query: SingleByIid<START_HALL_IID, Entity, With<ShieldtankWorldBounds>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    commands.insert_resource(CurrentLevel(*level_query));

    commands.trigger(AttemptSpawnLevel(LEVEL_UP));
    commands.trigger(AttemptSpawnLevel(LEVEL_RIGHT));
    commands.trigger(AttemptSpawnLevel(LEVEL_DOWN));
    commands.trigger(AttemptSpawnLevel(LEVEL_LEFT));

    next_state.set(GameState::Playing);
}

fn track_current_level(
    skeleton_location: SingleByIid<
        SKELETON_IID,
        &GlobalTransform,
        // ShieldtankLocationChanged
    >,
    level_query: QueryByGlobalBounds<
        (Entity, &Name, ShieldtankWorldLocation),
        With<ShieldtankLevel>,
    >,
    mut current_level: ResMut<CurrentLevel>,
    mut commands: Commands,
) {
    let skeleton_location = skeleton_location.translation().truncate();

    let Ok((level_under_skeleton, level_name, level_location)) =
        level_query.single_by_location(skeleton_location)
    else {
        info!("Skeleton is walking in space!");
        return;
    };

    if level_under_skeleton != **current_level {
        info!("Skeleton has wandered into a new level! {level_name}");
        **current_level = level_under_skeleton;

        let level_location = level_location.get();

        let level_code = parse_level_code(level_name);

        if level_code & WALL_UP == 0 {
            commands.trigger(AttemptSpawnLevel(level_location + LEVEL_UP));
        }

        if level_code & WALL_RIGHT == 0 {
            commands.trigger(AttemptSpawnLevel(level_location + LEVEL_RIGHT));
        }

        if level_code & WALL_DOWN == 0 {
            commands.trigger(AttemptSpawnLevel(level_location + LEVEL_DOWN));
        }

        if level_code & WALL_LEFT == 0 {
            commands.trigger(AttemptSpawnLevel(level_location + LEVEL_LEFT));
        }
    }
}

fn attempt_spawn_level(
    attemt_level_location: On<AttemptSpawnLevel>,
    level_query: QueryByGlobalBounds<&Name, With<ShieldtankLevel>>,
    asset_server: Res<AssetServer>,
    mut rand: Local<StdRand>,
    mut commands: Commands,
) {
    let attempt_level_location: Vec2 = **attemt_level_location;
    info!("Spawning new level at: {attempt_level_location}");

    const CENTER_OFFSET: Vec2 = Vec2::new(LEVEL_SIZE / 2.0, -LEVEL_SIZE / 2.0);

    if level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET)
        .is_ok()
    {
        return;
    }

    let level_up_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_UP)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    let level_right_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_RIGHT)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    let level_down_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_DOWN)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    let level_left_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_LEFT)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    let mut rand = rand.next_lim_u16(15);

    let mut fix_rand_by_code = |code: Option<u16>, wall: u16, opposite_wall: u16| {
        if let Some(code) = code {
            if code & opposite_wall == 0 {
                rand &= wall ^ 0xF;
            } else {
                rand |= wall;
            }
        }
    };

    fix_rand_by_code(level_up_code, WALL_UP, WALL_DOWN);
    fix_rand_by_code(level_right_code, WALL_RIGHT, WALL_LEFT);
    fix_rand_by_code(level_down_code, WALL_DOWN, WALL_UP);
    fix_rand_by_code(level_left_code, WALL_LEFT, WALL_RIGHT);

    let new_level_asset_label = format!("{PROJECT_FILE}#world:Dungeon/Level_{rand}");

    commands.spawn((
        ShieldtankLevel {
            handle: asset_server.load(new_level_asset_label),
            ..Default::default()
        },
        Transform::default().with_translation(attempt_level_location.extend(0.0)),
    ));
}

fn player_keyboard_commands(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    grid_query: GridValueQuery,
    level_query: QueryByGlobalBounds<(), With<ShieldtankLevel>>,
    mut skeleton_query: SingleByIid<
        SKELETON_IID,
        (
            &ShieldtankWorldBounds,
            &mut ShieldtankTile,
            ShieldtankWorldLocationMut,
        ),
        With<ShieldtankEntity>,
    >,
) {
    let (global_bounds, ref mut tile, ref mut location) = *skeleton_query;

    if !level_query.any(location.get()) {
        return;
    }

    let up_pressed = keyboard_input.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
    let right_pressed = keyboard_input.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    let down_pressed = keyboard_input.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);
    let left_pressed = keyboard_input.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);

    const KEY_UP: (bool, bool, bool, bool) = (true, false, false, false);
    const KEY_UP_RIGHT: (bool, bool, bool, bool) = (true, true, false, false);
    const KEY_RIGHT: (bool, bool, bool, bool) = (false, true, false, false);
    const KEY_DOWN_RIGHT: (bool, bool, bool, bool) = (false, true, true, false);
    const KEY_DOWN: (bool, bool, bool, bool) = (false, false, true, false);
    const KEY_DOWN_LEFT: (bool, bool, bool, bool) = (false, false, true, true);
    const KEY_LEFT: (bool, bool, bool, bool) = (false, false, false, true);
    const KEY_UP_LEFT: (bool, bool, bool, bool) = (true, false, false, true);

    let dir = (up_pressed, right_pressed, down_pressed, left_pressed);

    // Do we need to flip the sprite?
    match dir {
        KEY_UP_RIGHT | KEY_RIGHT | KEY_DOWN_RIGHT => tile.flip_x(false),
        KEY_UP_LEFT | KEY_LEFT | KEY_DOWN_LEFT => tile.flip_x(true),
        _ => {}
    };

    // Construct a direction vector
    let dir = match dir {
        KEY_UP => Vec2::new(0.0, 1.0),
        KEY_UP_RIGHT => Vec2::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2),
        KEY_RIGHT => Vec2::new(1.0, 0.0),
        KEY_DOWN_RIGHT => Vec2::new(FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
        KEY_DOWN => Vec2::new(0.0, -1.0),
        KEY_DOWN_LEFT => Vec2::new(-FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
        KEY_LEFT => Vec2::new(-1.0, 0.0),
        KEY_UP_LEFT => Vec2::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
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

fn main() {
    let log_plugin_settings = bevy::log::LogPlugin {
        level: bevy::log::Level::WARN,
        filter: "wgpu_hal=off,\
            winit=off,\
            bevy_winit=off,\
            calloop=off,\
            bevy_ldtk_asset=info,\
            shieldtank=info"
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
        Material2dPlugin::<BackgroundMaterial>::default(),
        ShieldtankPlugins,
    ));

    #[cfg(debug_assertions)]
    {
        use bevy_inspector_egui::bevy_egui::EguiPlugin;
        use bevy_inspector_egui::quick::WorldInspectorPlugin;
        app.add_plugins(EguiPlugin::default())
            .add_plugins(WorldInspectorPlugin::default());
    }

    app.init_state::<GameState>();

    app.add_systems(Startup, setup);

    app.add_systems(
        Update,
        wait_for_start_hall.run_if(in_state(GameState::Loading)),
    );

    app.add_systems(
        Update,
        (
            camera_follow_skeleton,
            camera_zoom_commands,
            track_current_level,
            player_keyboard_commands,
        )
            .run_if(in_state(GameState::Playing)),
    );

    app.add_observer(attempt_spawn_level);

    app.run();
}
