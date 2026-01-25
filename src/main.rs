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

const CLOUDS_SHADER_PATH: &str = "shaders/clouds.wesl";
const CLOUDS_Z: f32 = 900.0;

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

/// We track the level under the player in a resourse.
///
/// Used by [track_current_level]
#[derive(Resource, Deref, DerefMut)]
struct CurrentLevel(Entity);

/// When we move to another level, or when we start, we send this event for all
/// chunks in the directions north, east, south, and west.
///
/// This is handled by the [attempt_spawn_level] observer
#[derive(Event, Deref)]
struct AttemptSpawnLevel(Vec2);

/// The current state. This is a very simple state. We stay in
/// [GameState::Loading] until we find the start hall has been loaded.
/// Once loaded, [wait_for_start_hall] will send [AttemptSpawnLevel] messages
/// in the four cardinal directions and transition to [GameState::Playing],
/// where we remain until the player closes the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    Playing,
}

/// A custom 2d materal for drawing the clouds layer overlay.
///
/// Normally you wouldn't parameterize so many things, I just wanted to allow
/// people to play with the parameters in bevy_inspector_egui.
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
struct CloudsMaterial {
    #[uniform(0)]
    params0: Vec4,
    #[uniform(1)]
    params1: Vec4,
}

impl Default for CloudsMaterial {
    fn default() -> Self {
        Self {
            params0: Vec4::new(
                0.015, // Overall speed.
                2.0, 15.0, // Overall scale.
                0.0,  // The timer. Mutated in [cloud_material_update_time].
            ),
            params1: Vec4::new(
                0.5, // The speed multiplier for the left scrolling of the clouds.
                1.5, -0.25, // The linear transform we apply to the color.
                0.25,  // The alpha of the layer.
            ),
        }
    }
}

/// Implement [CloudsMaterial] as a [Material2d] with Bevy.
impl Material2d for CloudsMaterial {
    fn fragment_shader() -> ShaderRef {
        CLOUDS_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::sprite_render::AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker component for the clouds layer.
#[derive(Component)]
struct Clouds;

/// Convenience function for getting the `code` for the given level name.
///
/// Expected format:
/// - If the level name is `"Start_Hall"`, then return zero.
/// - Otherwise, we expect the level name to be in the form of `"Level_n"` where
///   _n_ is between 0 to 15.
fn parse_level_code(level_name: &str) -> u16 {
    match level_name {
        "Start_Hall" => 0,
        name if &name[0..6] == "Level_" => {
            let code = level_name[6..]
                .parse()
                .expect("Couldn't parse code: {rest}");
            if (0..15).contains(&code) {
                code
            } else {
                panic!("Code out of range! {code}")
            }
        }

        bad_name => panic!("bad level name! {bad_name}"),
    }
}

/// Spawn all the initial components:
/// - Camera
/// - Start Hall
/// - Text Label
/// - Clouds mesh
fn setup(
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudsMaterial>>,
    mut commands: Commands,
) {
    // The camera, initialized to the default scale. It's actually not where we
    // want, but when the skeleton spawns later on then it'll get moved.
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
        Name::new("Text Label"),
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

    // The clouds mesh.
    commands.spawn((
        Name::new("Clouds"),
        Mesh2d(meshes.add(Rectangle::from_corners(
            Vec2::splat(-9999.0),
            Vec2::splat(9999.0),
        ))),
        MeshMaterial2d(materials.add(CloudsMaterial::default())),
        Transform::from_translation(Vec2::ZERO.extend(CLOUDS_Z)),
        Clouds,
    ));
}

/// Update the mesh with the current game time.
fn cloud_material_update_time(
    time: Res<Time>,
    material: Single<&MeshMaterial2d<CloudsMaterial>>,
    mut materials: ResMut<Assets<CloudsMaterial>>,
) {
    let material = materials.get_mut(*material).unwrap();
    material.params0.w = time.elapsed_secs();
}

/// Move the camera and clouds mesh to always be centered over the player
/// skeleton.
///
/// This system won't run until the skeleton AND the cloud mesh are
/// fully loaded. This is fine for this demo, but a more serious game should
/// probably consider a more robust solution.
#[allow(clippy::type_complexity)]
fn camera_and_clouds_follow_skeleton(
    skeleton_transform: SingleByIid<
        SKELETON_IID,
        &Transform,
        (With<ShieldtankEntity>, Without<Camera2d>, Without<Clouds>),
    >,
    mut camera_transform: Single<
        &mut Transform,
        (Without<ShieldtankEntity>, With<Camera2d>, Without<Clouds>),
    >,
    mut cloud_transform: Single<
        &mut Transform,
        (Without<ShieldtankEntity>, Without<Camera2d>, With<Clouds>),
    >,
) {
    let skeleton_location = skeleton_transform.translation;

    let camera_z = camera_transform.translation.z;

    camera_transform.translation = skeleton_location.with_z(camera_z);
    cloud_transform.translation = skeleton_location.with_z(CLOUDS_Z);
}

/// Update the camera scale in response to the mouse wheel events.
fn camera_mouse_wheel_zoom(
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

/// This system will only run once. When the start hall is loaded and the
/// [ShieldtankWorldBounds] component is added:
/// - Create the [CurrentLevel] resource
/// - Send four [AttemptSpawnLevel] in each direction
/// - Change state to [GameState::Playing].
///
/// [attempt_spawn_level] will spawn levels in each direction, since there won't
/// be any there already.
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

/// Watch the skeleton location, and update the [CurrentLevel] resource if
/// changed.
///
/// Also, if the level changed, we send a [AttemptSpawnLevel] signal in each
/// of the four surrouding directions. [attempt_spawn_level] will spawn a level
/// there if the space is open.
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

/// The observer which responds to the [AttemptSpawnLevel] event.
fn attempt_spawn_level(
    attemt_level_location: On<AttemptSpawnLevel>,
    level_query: QueryByGlobalBounds<&Name, With<ShieldtankLevel>>,
    asset_server: Res<AssetServer>,
    mut rand: Local<StdRand>,
    mut commands: Commands,
) {
    // Coerce the event into the global `Vec2`.
    let attempt_level_location: Vec2 = **attemt_level_location;
    info!("Spawning new level at: {attempt_level_location}");

    // QueryByGlobalBounds is inclusive for all bounds, so we sample from the
    // center instead of attempt_level_location, which is the upper left corner.
    const CENTER_OFFSET: Vec2 = Vec2::new(LEVEL_SIZE / 2.0, -LEVEL_SIZE / 2.0);

    // If a level already exists here, we return early.
    if level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET)
        .is_ok()
    {
        return;
    }

    // `Some(code)` if there's a level to the north, `None` if there's no
    // level there (yet).
    let level_up_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_UP)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    // Similar, but to the east.
    let level_right_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_RIGHT)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    // Similar, but to the south.
    let level_down_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_DOWN)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    // Similar, but to the west.
    let level_left_code = level_query
        .single_by_location(attempt_level_location + CENTER_OFFSET + LEVEL_LEFT)
        .ok()
        .map(|level_name| parse_level_code(level_name));

    // Generate a new number from 0 to 15
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

    // If a level already exists in the given direction, explicitly set the
    // respective bit, so the doors match.
    fix_rand_by_code(level_up_code, WALL_UP, WALL_DOWN);
    fix_rand_by_code(level_right_code, WALL_RIGHT, WALL_LEFT);
    fix_rand_by_code(level_down_code, WALL_DOWN, WALL_UP);
    fix_rand_by_code(level_left_code, WALL_LEFT, WALL_RIGHT);

    // Spawn the new level, using the bevy_ldtk_asset asset path.
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

    // Flip the sprite based on the direction of movement.
    match dir {
        KEY_UP_RIGHT | KEY_RIGHT | KEY_DOWN_RIGHT => tile.flip_x(false),
        KEY_UP_LEFT | KEY_LEFT | KEY_DOWN_LEFT => tile.flip_x(true),
        _ => {}
    };

    // Construct a direction normal.
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

    // Calculate new location, with speed adjusted by `time.delta_secs()` so that
    // it's framerate independent.
    let new_location = location.get() + dir * time.delta_secs() * PLAYER_MOVE_SPEED;

    // Very crude collision detection.
    //
    // We create two sensors:
    // - One in the center of the line that makes up the left collision bound
    // - Another to the right.
    let rect = global_bounds.bounds();
    let half_size = rect.half_size();
    let half_width = half_size.x;
    let half_height = half_size.y;

    // If there is no grid value at the sample point, then it's passable.
    let sensor1 = Vec2::new(-half_width, half_height);
    let sensor1 = grid_query.grid_value_at(new_location + sensor1).is_none();

    let sensor2 = Vec2::new(half_width, half_height);
    let sensor2 = grid_query.grid_value_at(new_location + sensor2).is_none();

    // If both sensors are clear, go ahead and move the skeleton.
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
        Material2dPlugin::<CloudsMaterial>::default(),
        ShieldtankPlugins,
    ));

    #[cfg(debug_assertions)]
    {
        use bevy_inspector_egui::bevy_egui::EguiPlugin;
        use bevy_inspector_egui::quick::WorldInspectorPlugin;
        app.add_plugins(EguiPlugin::default())
            .add_plugins(WorldInspectorPlugin::default());
    }

    app.register_asset_reflect::<CloudsMaterial>();

    app.init_state::<GameState>();

    app.add_systems(Startup, setup);

    app.add_systems(
        Update,
        wait_for_start_hall.run_if(in_state(GameState::Loading)),
    );

    app.add_systems(
        Update,
        (
            cloud_material_update_time,
            camera_and_clouds_follow_skeleton,
            camera_mouse_wheel_zoom,
            track_current_level,
            player_keyboard_commands,
        )
            .run_if(in_state(GameState::Playing)),
    );

    app.add_observer(attempt_spawn_level);

    app.run();
}
