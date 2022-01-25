use crate::console_debug_plugin::Config;
use crate::console_debug_plugin::ConfigValue;
use bevy::{
    core::FixedTimestep,
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};
use noise::{HybridMulti, MultiFractal, NoiseFn};
use rand::prelude::random;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

pub struct AntsPlugin;

const TIME_STEP: f32 = 1.0 / 60.0;
const BOUNDS_X: f32 = 900.0;
const BOUNDS_Y: f32 = 600.0;
// TODO: move to config
const ANT_RANDOM_WANDERING: f32 = 0.02 * std::f32::consts::PI;
const OBSTACLE_TILE_SIZE: f32 = 10.0;
const OBSTACLE_COLOR: Color = Color::rgb(0.65, 0.16, 0.16);
const FOOD_SIZE: f32 = 5.0;
const FOOD_COLOR: Color = Color::rgb(0.0, 0.65, 0.0);
const TRAIL_SIZE: f32 = 2.5;
const TRAIL_GOT_FOOD_COLOR: Color = Color::rgb(0.88, 0.18, 0.24);
const TRAIL_GATHERING_COLOR: Color = Color::rgb(0.28, 0.51, 0.87);
const HOME_SIZE: f32 = 10.0;
const HOME_COLOR: Color = Color::rgb(1.0, 1.0, 0.62);
// TODO: fix ant size and scale
const ANT_SIZE: f32 = 5.0;

impl Plugin for AntsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Config>()
            .init_resource::<MapGenerator>()
            .init_resource::<EditorInput>()
            .add_startup_system(setup.label("setup"))
            .add_startup_system(map_generator_system.after("setup"))
            .add_system(mouse_input_system)
            .add_system(map_generator_system)
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                    .with_system(obstacle_collision_system)
                    .with_system(food_collision_system)
                    .with_system(ant_movement_system)
                    .with_system(trail_spawn_system)
                    .with_system(trail_decay_system),
            );
    }
}

#[derive(Component)]
struct Ant {
    carrying_food: bool,
}

#[derive(Component)]
enum Collider {
    Solid,
}

#[derive(Component, Copy, Clone)]
enum Icon {
    SpawnObstacle,
    SpawnFood,
    SpawnFoodCluster,
    SpawnHome,
}

#[derive(Component)]
struct Obstacle {}

#[derive(Component)]
struct Food {}

enum TrailType {
    Gathering,
    GotFood,
}

#[derive(Component)]
struct Trail {
    trail_type: TrailType,
    strength: f32,
}

#[derive(Component)]
struct Home {}

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
    config.entries.insert("ant.speed", ConfigValue::Float(40.0));
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
    config
        .entries
        .insert("trail.spawn_period", ConfigValue::Float(0.25));
    config
        .entries
        .insert("trail.initial_strength", ConfigValue::Float(1.0));
    config
        .entries
        .insert("trail.decay_rate", ConfigValue::Float(0.999));
    config.entries.insert(
        "sensor_angle",
        ConfigValue::Float(std::f32::consts::PI / 4.0),
    );
    config
        .entries
        .insert("sensor_distance", ConfigValue::Float(20.0));
    config
        .entries
        .insert("sensor_radius", ConfigValue::Float(7.66));
    config
        .entries
        .insert("sensor_turning_coefficient", ConfigValue::Float(1.0));

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    for _ in 0..10 {
        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("ant.png"),
                transform: Transform {
                    scale: Vec3::new(ANT_SIZE, ANT_SIZE, 0.0),
                    translation: Vec3::new(0.0, -50.0, 0.0),
                    rotation: Quat::from_rotation_z(random::<f32>() * 2.0 * std::f32::consts::PI),
                    ..Default::default()
                },
                sprite: Sprite {
                    custom_size: Some(Vec2::new(1.0, 1.0)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Ant {
                carrying_food: false,
            });
    }

    spawn_home(Vec3::new(0.0, -50.0, 0.0), &mut commands);

    spawn_food_cluster(Vec3::new(-218.0, -84.0, 0.0), &mut commands);
    spawn_food_cluster(Vec3::new(22.0, 157.0, 0.0), &mut commands);
    spawn_food_cluster(Vec3::new(235.0, 1.0, 0.0), &mut commands);

    // Add walls
    let wall_color = Color::rgb(0.8, 0.8, 0.8);
    let wall_thickness = 10.0;

    // left
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-(BOUNDS_X + wall_thickness) / 2.0, 0.0, 0.0),
                scale: Vec3::new(wall_thickness, BOUNDS_Y + wall_thickness * 2.0, 1.0),
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
                translation: Vec3::new((BOUNDS_X + wall_thickness) / 2.0, 0.0, 0.0),
                scale: Vec3::new(wall_thickness, BOUNDS_Y + wall_thickness * 2.0, 1.0),
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
                translation: Vec3::new(0.0, -(BOUNDS_Y + wall_thickness) / 2.0, 0.0),
                scale: Vec3::new(BOUNDS_X + wall_thickness * 2.0, wall_thickness, 1.0),
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
                translation: Vec3::new(0.0, (BOUNDS_Y + wall_thickness) / 2.0, 0.0),
                scale: Vec3::new(BOUNDS_X + wall_thickness * 2.0, wall_thickness, 1.0),
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
                translation: Vec3::new(-BOUNDS_X / 2.0 - 50.0, BOUNDS_Y / 2.0 - 15.0, 0.0),
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
                translation: Vec3::new(-BOUNDS_X / 2.0 - 50.0, BOUNDS_Y / 2.0 - 60.0, 0.0),
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
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-BOUNDS_X / 2.0 - 50.0, BOUNDS_Y / 2.0 - 105.0, 0.0),
                scale: Vec3::new(40.0, 40.0, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: FOOD_COLOR,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Icon::SpawnFoodCluster);
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-BOUNDS_X / 2.0 - 50.0, BOUNDS_Y / 2.0 - 150.0, 0.0),
                scale: Vec3::new(40.0, 40.0, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: HOME_COLOR,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Icon::SpawnHome);

    // // spawn some test trails
    // for _ in 0..1 {
    //     let foo = Vec3::new(9.0, 6.0, 0.0).normalize() * 25.0 * random::<f32>();
    //     let mut pos = Vec3::new(-450.0, -300.0, 0.0) + foo;
    //     while pos.x <= 450.0 && pos.y <= 300.0 {
    //         spawn_trail(
    //             pos,
    //             &mut commands,
    //             TrailType::Gathering,
    //             config.entries["trail.initial_strength"].f32(),
    //         );
    //         pos += Vec3::new(9.0, 6.0, 0.0).normalize() * 10.0;
    //     }
    // }
    // for _ in 0..5 {
    //     let step = Vec3::new(9.0, -6.0, 0.0).normalize() * 10.0;
    //     let foo = step * random::<f32>();
    //     let mut pos = Vec3::new(-450.0, 300.0, 0.0) + foo;
    //     while pos.x <= 450.0 && pos.y >= -300.0 {
    //         spawn_trail(
    //             pos,
    //             &mut commands,
    //             TrailType::Gathering,
    //             config.entries["trail.initial_strength"].f32(),
    //         );
    //         pos += step;
    //     }
    // }
}

fn window_to_world(position: Vec2, window: &Window, camera: &Transform) -> Vec3 {
    let norm = Vec3::new(
        position.x - window.width() / 2.,
        position.y - window.height() / 2.,
        0.,
    );
    let mut pos = *camera * norm;
    pos.z = 0.0;
    return pos;
}

fn to_tile_center(world_pos: Vec3, grid_size: f32) -> Vec3 {
    Vec3::new(
        (world_pos.x / grid_size).floor() * grid_size + grid_size / 2.0,
        (world_pos.y / grid_size).floor() * grid_size + grid_size / 2.0,
        0.0,
    )
}

fn pos_in_transform(pos: &Vec3, transform: &Transform) -> bool {
    pos.x > transform.translation.x - transform.scale.x / 2.0
        && pos.x < transform.translation.x + transform.scale.x / 2.0
        && pos.y > transform.translation.y - transform.scale.y / 2.0
        && pos.y < transform.translation.y + transform.scale.y / 2.0
}

fn pos_in_bounds(pos: &Vec3) -> bool {
    pos.x < BOUNDS_X / 2.0
        && pos.x > -BOUNDS_X / 2.0
        && pos.y < BOUNDS_Y / 2.0
        && pos.y > -BOUNDS_Y / 2.0
}

fn mouse_input_system(
    mut commands: Commands,
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut editor_input: ResMut<EditorInput>,
    transform_query: Query<&Transform, With<Camera>>,
    collider_query: Query<(Entity, &Collider, &Transform), Without<Ant>>,
    home_query: Query<(Entity, &Home, &Transform), Without<Ant>>,
    food_query: Query<(Entity, &Food, &Transform)>,
    icon_query: Query<(&Icon, &Transform)>,
) {
    let window = windows.get_primary().unwrap();
    if let Some(cursor_pos) = window.cursor_position() {
        let world_cursor_pos = window_to_world(cursor_pos, window, transform_query.single());

        if buttons.just_pressed(MouseButton::Left) {
            for (icon, transform) in icon_query.iter() {
                if pos_in_transform(&world_cursor_pos, &transform) {
                    editor_input.selected_icon = Some(*icon);
                }
            }
        }
        if pos_in_bounds(&world_cursor_pos) {
            match editor_input.selected_icon {
                Some(Icon::SpawnObstacle) => {
                    if buttons.pressed(MouseButton::Left) {
                        let tile_center = to_tile_center(world_cursor_pos, OBSTACLE_TILE_SIZE);
                        if !collider_query.iter().any(|(_, _, transform)| {
                            pos_in_transform(&world_cursor_pos, &transform)
                        }) {
                            spawn_obstacle(tile_center, &mut commands);
                        }
                    } else if buttons.pressed(MouseButton::Right) {
                        for (entity, _collider, transform) in collider_query.iter() {
                            if pos_in_transform(&world_cursor_pos, &transform) {
                                commands.entity(entity).despawn();
                            }
                        }
                    }
                }
                Some(Icon::SpawnFood) => {
                    if buttons.pressed(MouseButton::Left) {
                        if !food_query.iter().any(|(_, _, transform)| {
                            pos_in_transform(&world_cursor_pos, &transform)
                        }) {
                            spawn_food(world_cursor_pos.x, world_cursor_pos.y, &mut commands);
                        }
                    } else if buttons.pressed(MouseButton::Right) {
                        for (entity, _food, transform) in food_query.iter() {
                            if pos_in_transform(&world_cursor_pos, &transform) {
                                commands.entity(entity).despawn();
                            }
                        }
                    }
                }
                Some(Icon::SpawnFoodCluster) => {
                    if buttons.just_pressed(MouseButton::Left) {
                        if !food_query.iter().any(|(_, _, transform)| {
                            pos_in_transform(&world_cursor_pos, &transform)
                        }) {
                            spawn_food_cluster(world_cursor_pos, &mut commands);
                        }
                    } else if buttons.pressed(MouseButton::Right) {
                        for (entity, _food, transform) in food_query.iter() {
                            if pos_in_transform(&world_cursor_pos, &transform) {
                                commands.entity(entity).despawn();
                            }
                        }
                    }
                }
                Some(Icon::SpawnHome) => {
                    if buttons.pressed(MouseButton::Left) {
                        let tile_center = to_tile_center(world_cursor_pos, OBSTACLE_TILE_SIZE);
                        if !home_query.iter().any(|(_, _, transform)| {
                            pos_in_transform(&world_cursor_pos, &transform)
                        }) {
                            spawn_home(tile_center, &mut commands);
                        }
                    } else if buttons.pressed(MouseButton::Right) {
                        for (entity, _home, transform) in home_query.iter() {
                            if pos_in_transform(&world_cursor_pos, &transform) {
                                commands.entity(entity).despawn();
                            }
                        }
                    }
                }
                None => (),
            }
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
    let num_tiles_x = (BOUNDS_X / OBSTACLE_TILE_SIZE) as i32;
    let num_tiles_y = (BOUNDS_Y / OBSTACLE_TILE_SIZE) as i32;
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

fn spawn_food_cluster(pos: Vec3, commands: &mut Commands) {
    println!("food cluster: {}", pos);
    for i in 0..40 {
        let r = 20.0;
        let food_pos = pos
            + Vec3::new(
                random::<f32>() * 2.0 * r - r,
                random::<f32>() * 2.0 * r - r,
                0.0,
            );
        commands
            .spawn_bundle(SpriteBundle {
                transform: Transform {
                    translation: food_pos,
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
}

fn spawn_trail(pos: Vec3, commands: &mut Commands, trail_type: TrailType, initial_strength: f32) {
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: pos,
                scale: Vec3::new(TRAIL_SIZE, TRAIL_SIZE, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: match trail_type {
                    TrailType::GotFood => TRAIL_GOT_FOOD_COLOR,
                    TrailType::Gathering => TRAIL_GATHERING_COLOR,
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Trail {
            trail_type: trail_type,
            strength: initial_strength,
        });
}

fn spawn_home(pos: Vec3, commands: &mut Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: pos,
                scale: Vec3::new(HOME_SIZE, HOME_SIZE, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: HOME_COLOR,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Home {});
}

fn obstacle_collision_system(
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

fn food_collision_system(
    mut commands: Commands,
    mut ant_query: Query<(Entity, Option<&Children>, &mut Ant, &mut Transform), Without<Food>>,
    mut available_food_query: Query<
        (Entity, &Food, &mut Transform),
        (Without<Parent>, Without<Ant>),
    >,
    home_query: Query<(Entity, &Home, &Transform), (Without<Ant>, Without<Food>)>,
) {
    let mut taken_food: HashSet<u32> = HashSet::new();
    for (ant_entity, maybe_children, mut ant, mut ant_transform) in ant_query.iter_mut() {
        match maybe_children {
            Some(children) if children.len() > 0 => {
                // returning: check collision with home
                for (_home_entity, _home, transform) in home_query.iter() {
                    let a = ant_transform.translation.x - transform.translation.x;
                    let b = ant_transform.translation.y - transform.translation.y;
                    if a * a + b * b < HOME_SIZE * HOME_SIZE {
                        for &child in children.iter() {
                            commands.entity(child).despawn_recursive();
                        }
                        ant.carrying_food = false;
                        ant_transform.rotation *= Quat::from_rotation_z(std::f32::consts::PI);
                    }
                }
            }
            _ => {
                // gathering: check collision with food
                for (food_entity, _food, mut transform) in available_food_query.iter_mut() {
                    if taken_food.contains(&food_entity.id()) {
                        continue;
                    }
                    let a = ant_transform.translation.x - transform.translation.x;
                    let b = ant_transform.translation.y - transform.translation.y;
                    if a * a + b * b < FOOD_SIZE * FOOD_SIZE {
                        transform.scale = Vec3::new(1.0, 1.0, 1.0);
                        transform.translation = Vec3::new(1.0, 0.0, 0.0);
                        commands.entity(ant_entity).push_children(&[food_entity]);
                        taken_food.insert(food_entity.id());
                        ant.carrying_food = true;
                        ant_transform.rotation *= Quat::from_rotation_z(std::f32::consts::PI);
                        break;
                    }
                }
            }
        }
    }
}

fn trail_spawn_system(
    mut commands: Commands,
    mut current_frame: Local<usize>,
    config: Res<Config>,
    query: Query<(Entity, &Ant, &Transform)>,
) {
    let trail_spawn_period = config.entries["trail.spawn_period"].f32();
    let spawn_period_frames = (trail_spawn_period / TIME_STEP) as usize;
    let current_spawn_frame = *current_frame % spawn_period_frames;
    for (entity, ant, transform) in query.iter() {
        let mut rng = ChaCha8Rng::seed_from_u64(entity.id() as u64);
        let ant_spawn_frame_offset = rng.gen::<usize>() % spawn_period_frames;
        if ant_spawn_frame_offset == current_spawn_frame {
            spawn_trail(
                transform.translation,
                &mut commands,
                if ant.carrying_food {
                    TrailType::GotFood
                } else {
                    TrailType::Gathering
                },
                config.entries["trail.initial_strength"].f32(),
            );
        }
    }
    *current_frame += 1;
}

fn trail_decay_system(
    mut commands: Commands,
    config: Res<Config>,
    mut query: Query<(Entity, &mut Trail, &mut Sprite)>,
) {
    let initial_strength = config.entries["trail.initial_strength"].f32();
    let decay_rate = config.entries["trail.decay_rate"].f32();
    for (entity, mut trail, mut sprite) in query.iter_mut() {
        let mut color = match trail.trail_type {
            TrailType::GotFood => TRAIL_GOT_FOOD_COLOR,
            TrailType::Gathering => TRAIL_GATHERING_COLOR,
        };
        trail.strength = trail.strength * decay_rate;
        color.set_a(trail.strength / initial_strength);
        sprite.color = color;
        if trail.strength < 0.01 {
            commands.entity(entity).despawn();
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

fn ant_movement_system(
    mut ant_query: Query<(&Ant, &mut Transform), Without<Trail>>,
    trail_query: Query<(&Trail, &Transform), Without<Ant>>,
    config: Res<Config>,
) {
    let sensor_angle = config.entries["sensor_angle"].f32();
    let sensor_distance = config.entries["sensor_distance"].f32();
    let sensor_radius = config.entries["sensor_radius"].f32();
    let sensor_turning_coefficient = config.entries["sensor_turning_coefficient"].f32();
    let mut sensor_magnitudes = [0.0, 0.0, 0.0];
    let sensor_base_pos = Vec3::new(1.0 / ANT_SIZE, 0.0, 0.0) * sensor_distance;
    let sensor_positions = [
        Quat::from_rotation_z(sensor_angle) * sensor_base_pos,
        sensor_base_pos,
        Quat::from_rotation_z(-sensor_angle) * sensor_base_pos,
    ];
    for (ant, mut ant_transform) in ant_query.iter_mut() {
        let velocity = ant_transform.rotation * Vec3::X * config.entries["ant.speed"].f32();
        ant_transform.translation += velocity * TIME_STEP;

        let angle = vec3_angle(velocity);
        let wandering_angle_delta = ANT_RANDOM_WANDERING * (random::<f32>() * 2.0 - 1.0);

        let t_sensor_positions = [
            ant_transform.mul_vec3(sensor_positions[0]),
            ant_transform.mul_vec3(sensor_positions[1]),
            ant_transform.mul_vec3(sensor_positions[2]),
        ];
        for (trail, trail_transform) in trail_query.iter() {
            if ant.carrying_food && matches!(trail.trail_type, TrailType::Gathering)
                || !ant.carrying_food && matches!(trail.trail_type, TrailType::GotFood)
            {
                for (i, s_pos) in t_sensor_positions.iter().enumerate() {
                    if (trail_transform.translation - *s_pos).length() < sensor_radius {
                        sensor_magnitudes[i] += trail.strength;
                    }
                }
            }
        }
        let turning_direction = sensor_positions[0] * sensor_magnitudes[0]
            + sensor_positions[1] * sensor_magnitudes[1]
            + sensor_positions[2] * sensor_magnitudes[2];

        let turning_angle_delta = if turning_direction.length() > 0.0 {
            vec3_angle(turning_direction) * sensor_turning_coefficient
        } else {
            0.0
        };
        ant_transform.rotation =
            Quat::from_rotation_z(angle + turning_angle_delta + wandering_angle_delta);
    }
}
