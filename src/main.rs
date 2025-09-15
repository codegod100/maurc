use bevy::prelude::*;
use rand::Rng;

fn main() {
    console_error_panic_hook::set_once();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Bevy on Tauri WebView"),
                fit_canvas_to_parent: true,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .init_resource::<DragState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (spin, drag_rotate))
        .run();
}

#[derive(Component)]
struct Spinner {
    axis: Vec3,
    speed: f32,
}

#[derive(Resource, Default)]
struct DragState {
    dragging: bool,
    last_pos: Option<Vec2>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 3D camera with a light
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, -0.5, -0.25)),
        ..Default::default()
    });

    // Random axis
    let mut rng = rand::thread_rng();
    let axis = Vec3::new(
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
    )
    .normalize_or_zero();
    let axis = if axis.length_squared() == 0.0 { Vec3::Y } else { axis };

    // Cube mesh + material
    let mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.7, 1.0),
        ..Default::default()
    });

    commands.spawn((
        PbrBundle {
            mesh,
            material,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        },
        Spinner { axis, speed: 1.0 },
    ));
}

fn spin(time: Res<Time>, mut q: Query<(&mut Transform, &Spinner)>) {
    for (mut t, s) in &mut q {
        if s.axis.length_squared() > 0.0 {
            t.rotate(Quat::from_axis_angle(s.axis, s.speed * time.delta_seconds()));
        }
    }
}

fn drag_rotate(
    mut drag: ResMut<DragState>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut touch_events: EventReader<bevy::input::touch::TouchInput>,
    mut wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut q: Query<&mut Transform, With<Spinner>>,
    q_pos: Query<&GlobalTransform, With<Spinner>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let (camera, cam_transform) = camera_q.single();
    let pan_scale = 0.01; // world units per pixel
    let activate_radius = 80.0; // pixels

    let is_near_cube = |cursor: Vec2| -> bool {
        for gt in q_pos.iter() {
            let world = gt.translation();
            if let Some(screen) = camera.world_to_viewport(cam_transform, world) {
                if screen.distance(cursor) <= activate_radius {
                    return true;
                }
            }
        }
        false
    };

    // Mouse drag (desktop)
    if buttons.just_pressed(MouseButton::Left) {
        if let Some(cursor) = window.cursor_position() {
            if is_near_cube(cursor) {
                drag.dragging = true;
                drag.last_pos = Some(cursor);
            }
        }
    } else if buttons.just_released(MouseButton::Left) {
        drag.dragging = false;
        drag.last_pos = None;
    }

    if drag.dragging {
        if let (Some(prev), Some(curr)) = (drag.last_pos, window.cursor_position()) {
            let delta = curr - prev;
            for mut t in &mut q {
                t.translation.x += delta.x * pan_scale;
                t.translation.y -= delta.y * pan_scale;
            }
            drag.last_pos = Some(curr);
        }
    }

    // Touch drag (mobile)
    for ev in touch_events.read() {
        match ev.phase {
            bevy::input::touch::TouchPhase::Started => {
                if is_near_cube(ev.position) {
                    drag.dragging = true;
                    drag.last_pos = Some(ev.position);
                }
            }
            bevy::input::touch::TouchPhase::Moved => {
                if let Some(prev) = drag.last_pos {
                    let curr = ev.position;
                    let delta = curr - prev;
                    for mut t in &mut q {
                        t.translation.x += delta.x * pan_scale;
                        t.translation.y -= delta.y * pan_scale;
                    }
                    drag.last_pos = Some(curr);
                }
            }
            bevy::input::touch::TouchPhase::Ended | bevy::input::touch::TouchPhase::Canceled => {
                drag.dragging = false;
                drag.last_pos = None;
            }
        }
    }

    // Mouse wheel to zoom (move object along z)
    for ev in wheel_events.read() {
        let scroll = ev.y;
        for mut t in &mut q {
            t.translation.z = (t.translation.z - scroll * 0.1).clamp(-10.0, 10.0);
        }
    }
}
