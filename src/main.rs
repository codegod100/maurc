use bevy::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::render::texture::ImagePlugin;
use bevy::render::view::Msaa;
use bevy::utils::Instant;
use rand::Rng;

// --- Game tuning constants ---
const TRACK_HALF_X: f32 = 4.2; // world units half-width for movement
const PLAYER_Z: f32 = 0.0;
const PLAYER_SIZE: Vec3 = Vec3::new(0.8, 0.8, 0.8);
const OBSTACLE_SIZE: Vec3 = Vec3::new(0.8, 0.8, 0.8);
const OBSTACLE_START_Z: f32 = -25.0;
const OBSTACLE_DESPAWN_Z: f32 = 7.0;
const OBSTACLE_SPEED: f32 = 8.0; // units/sec towards camera
const SPAWN_EVERY: f32 = 0.9;    // seconds
const DRAG_X_PER_PX: f32 = 0.02; // world units per horizontal pixel drag
const PLAYER_LERP_SPEED: f32 = 12.0; // x-axis smoothing towards target
const KEY_STEP_X: f32 = 0.9; // keyboard step per press

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    #[default]
    Menu,
    Playing,
    GameOver,
}

#[derive(Component)]
struct Player {
    target_x: f32,
}

#[derive(Component)]
struct Obstacle;

#[derive(Component)]
struct Warmup;

#[derive(Resource, Default)]
struct Score {
    value: f32,
    best: f32,
}

#[derive(Resource)]
struct SpawnTimer(Timer);

#[derive(Resource, Default)]
struct TouchState {
    active_id: Option<u64>,
    anchor: Option<Vec2>,
}

#[derive(Resource, Clone, Copy)]
struct AppBootTime {
    app_start: Instant,
    first_update_logged: bool,
}

#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct MenuUi;
#[derive(Component)]
struct GameOverUi;
#[derive(Component)]
struct HudRoot;

fn main() {
    console_error_panic_hook::set_once();

    // Signal as soon as WASM enters main (requested behavior)
    dispatch_bevy_ready_event();

    let start = Instant::now();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: String::from("Lane Runner (Bevy)"),
                        fit_canvas_to_parent: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest())
        )
        .insert_resource(Msaa::Off)
        .insert_resource(AppBootTime { app_start: start, first_update_logged: false })
        .init_state::<GameState>()
        .insert_resource(Score::default())
        .insert_resource(SpawnTimer(Timer::from_seconds(
            SPAWN_EVERY,
            TimerMode::Repeating,
        )))
        .insert_resource(TouchState::default())
        // world setup
        .add_systems(Startup, setup)
        .add_systems(Startup, log_after_setup)
        // Menu
        .add_systems(OnEnter(GameState::Menu), enter_menu)
        .add_systems(Update, (menu_start, first_update_probe).run_if(in_state(GameState::Menu)))
        .add_systems(OnExit(GameState::Menu), exit_menu)
        // Playing
        .add_systems(OnEnter(GameState::Playing), enter_playing)
        .add_systems(
            Update,
            (
                player_input,
                update_player_transform,
                spawn_obstacles,
                move_obstacles,
                collision_system,
                score_system,
                update_score_text,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(OnExit(GameState::Playing), exit_playing)
        // GameOver
        .add_systems(OnEnter(GameState::GameOver), enter_game_over)
        .add_systems(Update, game_over_restart.run_if(in_state(GameState::GameOver)))
        .add_systems(OnExit(GameState::GameOver), exit_game_over)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    bt: Res<AppBootTime>,
) {
    info!("[boot] setup: begin (+{:?} since start)", bt.app_start.elapsed());
    // Camera slightly above and behind, looking at the play area
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6.0, 8.0)
            .looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
        camera: Camera { hdr: false, ..Default::default() },
        tonemapping: Tonemapping::None,
        ..Default::default()
    });

    // Prewarm PBR pipeline with an off-screen unlit cube
    let warm_mesh = meshes.add(Mesh::from(Cuboid::new(0.1, 0.1, 0.1)));
    let warm_mat = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 1.0, 1.0), unlit: true, ..Default::default() });
    commands.spawn((
        PbrBundle {
            mesh: warm_mesh,
            material: warm_mat,
            transform: Transform::from_xyz(0.0, -1000.0, 0.0),
            ..Default::default()
        },
        Warmup,
    ));

    info!("[boot] setup: end (+{:?})", bt.app_start.elapsed());
}

#[cfg(target_arch = "wasm32")]
fn dispatch_bevy_ready_event() {
    use wasm_bindgen::JsCast;
    use web_sys::{Event, Window};
    let window: Window = web_sys::window().expect("no global `window` exists");
    let ev = Event::new("bevy_ready").unwrap();
    let _ = window.dispatch_event(&ev);
}
#[cfg(not(target_arch = "wasm32"))]
fn dispatch_bevy_ready_event() {}

fn log_after_setup(bt: Res<AppBootTime>) {
    info!("[boot] startup stage done (+{:?})", bt.app_start.elapsed());
    dispatch_bevy_ready_event();
}

fn first_update_probe(mut bt: ResMut<AppBootTime>) {
    if !bt.first_update_logged {
        info!("[boot] first update tick (+{:?})", bt.app_start.elapsed());
        bt.first_update_logged = true;
    }
}

// --- Menu ---
fn enter_menu(mut commands: Commands, bt: Res<AppBootTime>) {
    info!("[boot] menu: enter (+{:?})", bt.app_start.elapsed());
    // Full-screen centered "Tap to Start"
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..Default::default()
            },
            MenuUi,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Tap to Start",
                TextStyle {
                    font_size: 42.0,
                    color: Color::WHITE,
                    ..Default::default()
                },
            ));
        });
}

fn menu_start(
    mut touch_evs: EventReader<TouchInput>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    bt: Res<AppBootTime>,
) {
    let touched = touch_evs.read().next().is_some();
    let clicked = mouse.just_pressed(MouseButton::Left);
    let keyed = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);

    if touched || clicked || keyed {
        info!("[boot] menu: input -> request Playing (+{:?})", bt.app_start.elapsed());
        next_state.set(GameState::Playing);
    }
}

fn exit_menu(mut commands: Commands, q: Query<Entity, With<MenuUi>>, bt: Res<AppBootTime>) {
    info!("[boot] menu: exit (+{:?})", bt.app_start.elapsed());
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

// --- Playing ---
fn enter_playing(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut score: ResMut<Score>,
    mut spawn_timer: ResMut<SpawnTimer>,
    bt: Res<AppBootTime>,
) {
    info!("[boot] playing: enter (+{:?})", bt.app_start.elapsed());
    // Reset score and timer
    score.value = 0.0;
    spawn_timer.0.reset();

    // Player
    let player_mesh = meshes.add(Mesh::from(Cuboid::new(
        PLAYER_SIZE.x,
        PLAYER_SIZE.y,
        PLAYER_SIZE.z,
    )));
    let player_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.9, 0.3),
        unlit: true,
        ..Default::default()
    });

    commands.spawn((
        PbrBundle {
            mesh: player_mesh.clone(),
            material: player_mat.clone(),
            transform: Transform::from_xyz(0.0, PLAYER_SIZE.y * 0.5, PLAYER_Z),
            ..Default::default()
        },
        Player { target_x: 0.0 },
    ));

    // Ground
    let ground_mesh = meshes.add(Mesh::from(Cuboid::new(10.0, 0.1, 60.0)));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.16),
        unlit: true,
        ..Default::default()
    });
    commands.spawn(PbrBundle {
        mesh: ground_mesh,
        material: ground_mat,
        transform: Transform::from_xyz(0.0, -0.05, -10.0),
        ..Default::default()
    });

    // HUD (score)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(16.0),
                    left: Val::Px(16.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            HudRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "Score: 0",
                    TextStyle {
                        font_size: 28.0,
                        color: Color::WHITE,
                        ..Default::default()
                    },
                ),
                ScoreText,
            ));
        });
}

fn player_input(
    mut q_player: Query<(&Transform, &mut Player)>,
    keys: Res<ButtonInput<KeyCode>>,
    mut touch_evs: EventReader<TouchInput>,
    mut touch_state: ResMut<TouchState>,
) {
    // Keyboard (desktop): discrete steps
    for (_t, mut p) in &mut q_player {
        if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
            p.target_x = (p.target_x - KEY_STEP_X).clamp(-TRACK_HALF_X, TRACK_HALF_X);
        }
        if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
            p.target_x = (p.target_x + KEY_STEP_X).clamp(-TRACK_HALF_X, TRACK_HALF_X);
        }
    }

    // Touch drag (mobile): continuous mapping
    for ev in touch_evs.read() {
        match ev.phase {
            TouchPhase::Started => {
                if touch_state.active_id.is_none() {
                    touch_state.active_id = Some(ev.id);
                    touch_state.anchor = Some(ev.position);
                }
            }
            TouchPhase::Moved => {
                if touch_state.active_id == Some(ev.id) {
                    if let Some(anchor) = touch_state.anchor {
                        let dx_px = ev.position.x - anchor.x;
                        // Update anchor so movement is incremental
                        touch_state.anchor = Some(ev.position);
                        for (_t, mut p) in &mut q_player {
                            p.target_x = (p.target_x + dx_px * DRAG_X_PER_PX)
                                .clamp(-TRACK_HALF_X, TRACK_HALF_X);
                        }
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                if touch_state.active_id == Some(ev.id) {
                    touch_state.active_id = None;
                    touch_state.anchor = None;
                }
            }
        }
    }
}

fn update_player_transform(time: Res<Time>, mut q: Query<(&Player, &mut Transform)>) {
    for (p, mut t) in &mut q {
        let target_x = p.target_x;
        let dx = target_x - t.translation.x;
        let step = PLAYER_LERP_SPEED * time.delta_seconds();
        if dx.abs() <= step {
            t.translation.x = target_x;
        } else if dx > 0.0 {
            t.translation.x += step;
        } else {
            t.translation.x -= step;
        }
    }
}

fn spawn_obstacles(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<SpawnTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    bt: Res<AppBootTime>,
    mut first_spawn_logged: Local<bool>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(-TRACK_HALF_X..=TRACK_HALF_X);

        let mesh = meshes.add(Mesh::from(Cuboid::new(
            OBSTACLE_SIZE.x,
            OBSTACLE_SIZE.y,
            OBSTACLE_SIZE.z,
        )));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.3, 0.3),
            unlit: true,
            ..Default::default()
        });

        commands.spawn((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_xyz(
                    x,
                    OBSTACLE_SIZE.y * 0.5,
                    OBSTACLE_START_Z,
                ),
                ..Default::default()
            },
            Obstacle,
        ));

        if !*first_spawn_logged {
            info!("[boot] first obstacle spawned (+{:?})", bt.app_start.elapsed());
            *first_spawn_logged = true;
        }
    }
}

fn move_obstacles(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Transform), With<Obstacle>>) {
    for (e, mut t) in &mut q {
        t.translation.z += OBSTACLE_SPEED * time.delta_seconds();
        if t.translation.z > OBSTACLE_DESPAWN_Z {
            commands.entity(e).despawn();
        }
    }
}

fn collision_system(
    mut next_state: ResMut<NextState<GameState>>,
    mut score: ResMut<Score>,
    q_player: Query<&Transform, With<Player>>,
    q_obstacles: Query<&Transform, With<Obstacle>>,
) {
    let Ok(player_t) = q_player.get_single() else { return; };

    let px = player_t.translation.x;
    let pz = player_t.translation.z;

    // Simple AABB overlap check on X and Z
    let half_x = (PLAYER_SIZE.x + OBSTACLE_SIZE.x) * 0.5 * 0.8; // generous overlap
    let half_z = (PLAYER_SIZE.z + OBSTACLE_SIZE.z) * 0.5 * 0.8;

    for ot in &q_obstacles {
        let dx = (ot.translation.x - px).abs();
        let dz = (ot.translation.z - pz).abs();
        if dx < half_x && dz < half_z {
            // Game over
            if score.value > score.best {
                score.best = score.value;
            }
            next_state.set(GameState::GameOver);
            break;
        }
    }
}

fn score_system(time: Res<Time>, mut score: ResMut<Score>) {
    score.value += time.delta_seconds() * 10.0;
}

fn update_score_text(score: Res<Score>, mut q: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() { return; }
    for mut text in &mut q {
        text.sections[0].value = format!("Score: {}", score.value as i32);
    }
}

fn exit_playing(
    mut commands: Commands,
    q_player: Query<Entity, With<Player>>,
    q_obstacles: Query<Entity, With<Obstacle>>,
    q_hud: Query<Entity, With<HudRoot>>,
) {
    for e in &q_player {
        commands.entity(e).despawn_recursive();
    }
    for e in &q_obstacles {
        commands.entity(e).despawn_recursive();
    }
    for e in &q_hud {
        commands.entity(e).despawn_recursive();
    }
}

// --- Game Over ---
fn enter_game_over(mut commands: Commands, score: Res<Score>) {
    let msg = format!("Game Over\nScore: {}  Best: {}\nTap to Restart",
        score.value as i32, score.best as i32);

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..Default::default()
            },
            GameOverUi,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                msg,
                TextStyle {
                    font_size: 36.0,
                    color: Color::WHITE,
                    ..Default::default()
                },
            ));
        });
}

fn game_over_restart(
    mut touch_evs: EventReader<TouchInput>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let touched = touch_evs.read().next().is_some();
    let clicked = mouse.just_pressed(MouseButton::Left);
    let keyed = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);

    if touched || clicked || keyed {
        next_state.set(GameState::Playing);
    }
}

fn exit_game_over(mut commands: Commands, q: Query<Entity, With<GameOverUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
