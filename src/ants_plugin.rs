use crate::console_debug_plugin::Config;
use crate::console_debug_plugin::ConfigValue;
use bevy::{
    core::FixedTimestep,
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};
use noise::{HybridMulti, MultiFractal, NoiseFn};
use rand::prelude::random;

pub struct AntsPlugin;

const TIME_STEP: f32 = 1.0 / 60.0;
const ANT_RANDOM_WANDERING: f32 = 0.5;
const OBSTACLE_TILE_SIZE: f32 = 10.0;
const OBSTACLE_COLOR: Color = Color::rgb(0.65, 0.16, 0.16);
const FOOD_SIZE: f32 = 5.0;
const FOOD_COLOR: Color = Color::rgb(0.0, 0.65, 0.0);

impl Plugin for AntsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Config>()
            .init_resource::<MapGenerator>()
            .init_resource::<EditorInput>()
            .add_startup_system(setup.label("setup"))
            .add_startup_system(map_generator_system.after("setup"))
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                    .with_system(mouse_input_system)
                    .with_system(ant_collision_system)
                    .with_system(ant_movement_system)
                    .with_system(map_generator_system),
            );
    }
}

#[derive(Component)]
struct Ant {}

#[derive(Component)]
enum Collider {
    Solid,
}

#[derive(Component, Copy, Clone)]
enum Icon {
    SpawnObstacle,
    SpawnFood,
}

#[derive(Component)]
struct Obstacle {}

#[derive(Component)]
struct Food {}

#[derive(Default)]
struct MapGenerator {
    octaves: usize,
    frequency: f64,
    lacunarity: f64,
    persistence: f64,
    threshold: f64,
}

#[derive(Default)]
struct EditorInput {
    selected_icon: Option<Icon>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut config: ResMut<Config>) {
    config.entries.insert("ant.speed", ConfigValue::Float(50.0));
    config.entries.insert("map.octaves", ConfigValue::Int(4));
    config
        .entries
        .insert("map.frequency", ConfigValue::Float(0.005));
    config
        .entries
        .insert("map.lacunarity", ConfigValue::Float(1.0));
    config
        .entries
        .insert("map.persistence", ConfigValue::Float(0.2));
    config
        .entries
        .insert("map.threshold", ConfigValue::Float(0.4));

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    for _ in 0..100 {
        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("ant.png"),
                transform: Transform {
                    scale: Vec3::new(5.0, 5.0, 0.0),
                    translation: Vec3::new(0.0, -50.0, 1.0),
                    rotation: Quat::from_rotation_z(random::<f32>() * 2.0 * std::f32::consts::PI),
                    ..Default::default()
                },
                sprite: Sprite {
                    custom_size: Some(Vec2::new(1.0, 1.0)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Ant {});
    }

    // Add walls
    let wall_color = Color::rgb(0.8, 0.8, 0.8);
    let wall_thickness = 10.0;
    let bounds = Vec2::new(900.0, 600.0);

    // left
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-(bounds.x + wall_thickness) / 2.0, 0.0, 0.0),
                scale: Vec3::new(wall_thickness, bounds.y + wall_thickness * 2.0, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: wall_color,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid);
    // right
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new((bounds.x + wall_thickness) / 2.0, 0.0, 0.0),
                scale: Vec3::new(wall_thickness, bounds.y + wall_thickness * 2.0, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: wall_color,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid);
    // bottom
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, -(bounds.y + wall_thickness) / 2.0, 0.0),
                scale: Vec3::new(bounds.x + wall_thickness * 2.0, wall_thickness, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: wall_color,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid);
    // top
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, (bounds.y + wall_thickness) / 2.0, 0.0),
                scale: Vec3::new(bounds.x + wall_thickness * 2.0, wall_thickness, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: wall_color,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid);

    // icons
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-bounds.x / 2.0 - 50.0, bounds.y / 2.0 - 15.0, 0.0),
                scale: Vec3::new(40.0, 40.0, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: OBSTACLE_COLOR,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Icon::SpawnObstacle);
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-bounds.x / 2.0 - 50.0, bounds.y / 2.0 - 60.0, 0.0),
                scale: Vec3::new(40.0, 40.0, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: FOOD_COLOR,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Icon::SpawnFood);
}

fn window_to_world(position: Vec2, window: &Window, camera: &Transform) -> Vec3 {
    let norm = Vec3::new(
        position.x - window.width() / 2.,
        position.y - window.height() / 2.,
        0.,
    );
    *camera * norm
}

fn to_tile_center(world_pos: Vec3, grid_size: f32) -> Vec3 {
    Vec3::new(
        (world_pos.x / grid_size).floor() * grid_size + grid_size / 2.0,
        (world_pos.y / grid_size).floor() * grid_size + grid_size / 2.0,
        0.0,
    )
}

fn mouse_input_system(
    mut commands: Commands,
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut editor_input: ResMut<EditorInput>,
    transform_query: Query<&Transform, With<Camera>>,
    collider_query: Query<(Entity, &Collider, &Transform), Without<Ant>>,
    food_query: Query<(Entity, &Food, &Transform)>,
    icon_query: Query<(&Icon, &Transform)>,
) {
    let window = windows.get_primary().unwrap();
    if let Some(cursor_pos) = window.cursor_position() {
        let world_cursor_pos = window_to_world(cursor_pos, window, transform_query.single());

        if buttons.just_pressed(MouseButton::Left) {
            for (icon, transform) in icon_query.iter() {
                if world_cursor_pos.x > transform.translation.x - transform.scale.x / 2.0
                    && world_cursor_pos.x < transform.translation.x + transform.scale.x / 2.0
                    && world_cursor_pos.y > transform.translation.y - transform.scale.y / 2.0
                    && world_cursor_pos.y < transform.translation.y + transform.scale.y / 2.0
                {
                    editor_input.selected_icon = Some(*icon);
                }
            }
        }

        match editor_input.selected_icon {
            Some(Icon::SpawnObstacle) => {
                if buttons.pressed(MouseButton::Left) {
                    let tile_center = to_tile_center(world_cursor_pos, OBSTACLE_TILE_SIZE);
                    spawn_obstacle(tile_center, &mut commands);
                } else if buttons.pressed(MouseButton::Right) {
                    for (entity, _collider, transform) in collider_query.iter() {
                        if world_cursor_pos.x > transform.translation.x - transform.scale.x / 2.0
                            && world_cursor_pos.x
                                < transform.translation.x + transform.scale.x / 2.0
                            && world_cursor_pos.y
                                > transform.translation.y - transform.scale.y / 2.0
                            && world_cursor_pos.y
                                < transform.translation.y + transform.scale.y / 2.0
                        {
                            commands.entity(entity).despawn();
                        }
                    }
                }
            }
            Some(Icon::SpawnFood) => {
                if buttons.pressed(MouseButton::Left) {
                    spawn_food(world_cursor_pos.x, world_cursor_pos.y, &mut commands);
                } else if buttons.pressed(MouseButton::Right) {
                    for (entity, _food, transform) in food_query.iter() {
                        if world_cursor_pos.x > transform.translation.x - transform.scale.x / 2.0
                            && world_cursor_pos.x
                                < transform.translation.x + transform.scale.x / 2.0
                            && world_cursor_pos.y
                                > transform.translation.y - transform.scale.y / 2.0
                            && world_cursor_pos.y
                                < transform.translation.y + transform.scale.y / 2.0
                        {
                            commands.entity(entity).despawn();
                        }
                    }
                }
            }
            None => (),
        }
    }
}

fn map_generator_system(
    mut commands: Commands,
    config: ResMut<Config>,
    mut map_generator: ResMut<MapGenerator>,
    obstacle_query: Query<Entity, With<Obstacle>>,
) {
    if map_generator.octaves == config.entries["map.octaves"].usize()
        && map_generator.frequency == config.entries["map.frequency"].f64()
        && map_generator.lacunarity == config.entries["map.lacunarity"].f64()
        && map_generator.persistence == config.entries["map.persistence"].f64()
        && map_generator.threshold == config.entries["map.threshold"].f64()
    {
        return;
    }
    map_generator.octaves = config.entries["map.octaves"].usize();
    map_generator.frequency = config.entries["map.frequency"].f64();
    map_generator.lacunarity = config.entries["map.lacunarity"].f64();
    map_generator.persistence = config.entries["map.persistence"].f64();
    map_generator.threshold = config.entries["map.threshold"].f64();
    let noise = HybridMulti::new()
        .set_octaves(map_generator.octaves)
        .set_frequency(map_generator.frequency)
        .set_lacunarity(map_generator.lacunarity)
        .set_persistence(map_generator.persistence);

    for entity in obstacle_query.iter() {
        commands.entity(entity).despawn();
    }

    // obstacles
    let bounds = Vec2::new(900.0, 600.0);
    let num_tiles_x = (bounds.x / OBSTACLE_TILE_SIZE) as i32;
    let num_tiles_y = (bounds.y / OBSTACLE_TILE_SIZE) as i32;
    for i in 0..num_tiles_x {
        for j in 0..num_tiles_y {
            let ox = OBSTACLE_TILE_SIZE * ((i - num_tiles_x / 2) as f32) + OBSTACLE_TILE_SIZE / 2.0;
            let oy = OBSTACLE_TILE_SIZE * ((num_tiles_y / 2 - j) as f32) - OBSTACLE_TILE_SIZE / 2.0;
            if noise.get([ox as f64, oy as f64]) < map_generator.threshold {
                continue;
            }
            spawn_obstacle(Vec3::new(ox, oy, 0.0), &mut commands);
        }
    }
}

fn spawn_obstacle(pos: Vec3, commands: &mut Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: pos,
                scale: Vec3::new(OBSTACLE_TILE_SIZE, OBSTACLE_TILE_SIZE, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: OBSTACLE_COLOR,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid)
        .insert(Obstacle {});
}

fn spawn_food(x: f32, y: f32, commands: &mut Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(x, y, 0.0),
                scale: Vec3::new(FOOD_SIZE, FOOD_SIZE, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: FOOD_COLOR,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Food {});
}

fn ant_collision_system(
    mut ant_query: Query<(&Ant, &mut Transform), Without<Collider>>,
    collider_query: Query<(&Collider, &Transform), Without<Ant>>,
) {
    for (_ant, mut ant_transform) in ant_query.iter_mut() {
        let ant_size = ant_transform.scale.truncate();

        // check collision with walls
        for (_collider, transform) in collider_query.iter() {
            let a = ant_transform.translation.x - transform.translation.x;
            let b = ant_transform.translation.y - transform.translation.y;
            let c = transform.scale.x.max(transform.scale.y);
            if a * a + b * b < c * c {
                let collision = collide(
                    ant_transform.translation,
                    ant_size,
                    transform.translation,
                    transform.scale.truncate(),
                );
                if let Some(collision) = collision {
                    // reflect the ball when it collides
                    let mut reflect_x = false;
                    let mut reflect_y = false;
                    let direction = ant_transform.rotation * Vec3::X;

                    // only reflect if the ball's velocity is going in the opposite direction of the
                    // collision
                    match collision {
                        Collision::Left => reflect_x = direction.x > 0.0,
                        Collision::Right => reflect_x = direction.x < 0.0,
                        Collision::Top => reflect_y = direction.y < 0.0,
                        Collision::Bottom => reflect_y = direction.y > 0.0,
                    }

                    // reflect velocity on the x-axis if we hit something on the x-axis
                    if reflect_x {
                        let clamped_direction = Vec3::new(-direction.x * 0.1, direction.y, 0.0);
                        let angle = vec3_angle(clamped_direction);
                        ant_transform.rotation = Quat::from_rotation_z(angle);
                    }

                    // reflect velocity on the y-axis if we hit something on the y-axis
                    if reflect_y {
                        let clamped_direction = Vec3::new(direction.x, -direction.y * 0.1, 0.0);
                        let angle = vec3_angle(clamped_direction);
                        ant_transform.rotation = Quat::from_rotation_z(angle);
                    }
                }
            }
        }
    }
}

fn vec3_angle(v: Vec3) -> f32 {
    let angle = v.angle_between(Vec3::X);
    if v.y < 0.0 {
        -angle
    } else {
        angle
    }
}

fn ant_movement_system(mut query: Query<&mut Transform, With<Ant>>, config: Res<Config>) {
    for mut transform in query.iter_mut() {
        let velocity = transform.rotation * Vec3::X * config.entries["ant.speed"].f32();
        transform.translation += velocity * TIME_STEP;

        let angle = vec3_angle(velocity);
        let wandering_angle_delta = ANT_RANDOM_WANDERING * (random::<f32>() - 0.5);
        transform.rotation = Quat::from_rotation_z(angle + wandering_angle_delta);
    }
}
