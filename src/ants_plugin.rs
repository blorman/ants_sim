use crate::console_debug_plugin::Config;
use crate::console_debug_plugin::ConfigValue;
use crate::helpers::tilemap_utils::*;
use bevy::{
    core::FixedTimestep,
    prelude::*,
    render::render_resource::TextureUsages,
    sprite::collide_aabb::{collide, Collision},
};
use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::prelude::*;
use nalgebra::{Point2, Vector2};
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
        app.add_plugin(TilemapPlugin)
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugin(RapierRenderPlugin)
            .init_resource::<Config>()
            .init_resource::<MapGenerator>()
            .init_resource::<EditorInput>()
            .insert_resource(RapierConfiguration {
                scale: 5.0,
                gravity: Vector::new(0.0, 0.0),
                ..Default::default()
            })
            .add_startup_system(setup.label("setup"))
            .add_startup_system(map_generator_system.after("setup"))
            .add_system(mouse_input_system)
            .add_system(map_generator_system)
            .add_system(set_texture_filters_to_nearest)
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                    .with_system(obstacle_collision_system)
                    .with_system(food_collision_system)
                    .with_system(ant_movement_system)
                    .with_system(ant_movement_system2)
                    .with_system(trail_spawn_system)
                    .with_system(trail_decay_system),
            );
    }
}

#[derive(Component)]
struct Ant {
    carrying_food: bool,
    target_speed: f32,
    motor_force: f32,
    grip_force: f32,
    turning_torque: f32,
    random_turning_torque: f32,
}

impl Default for Ant {
    fn default() -> Ant {
        Ant {
            carrying_food: false,
            target_speed: 8.0,
            motor_force: 4.0,
            grip_force: 5.0,
            turning_torque: 2.0,
            random_turning_torque: 5.0,
        }
    }
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
    config.entries.insert(
        "ant.wandering",
        ConfigValue::Float(0.02 * std::f32::consts::PI),
    );
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
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scale = 1.0;
    commands.spawn_bundle(camera);

    // spawn ants
    for _ in 0..1 {
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
                ..Default::default()
            });
    }

    /* Create a parallel rapier ant */
    let rigid_body = RigidBodyBundle {
        position: Vec2::new(0.0, 1.0).into(),
        damping: RigidBodyDamping {
            linear_damping: 2.0,
            angular_damping: 5.0,
        }
        .into(),
        ..Default::default()
    };
    let collider = ColliderBundle {
        // TODO: debug render a capsule?
        shape: ColliderShape::capsule(Point2::new(0.25, 0.0), Point2::new(-0.25, 0.0), 0.25).into(),
        // shape: ColliderShape::cuboid(0.5, 0.25).into(),
        material: ColliderMaterial {
            restitution: 0.7,
            friction: 0.0,
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };
    commands
        .spawn_bundle(rigid_body)
        .insert_bundle(collider)
        .insert_bundle(SpriteBundle {
            texture: asset_server.load("ant.png"),
            transform: Transform {
                scale: Vec3::new(ANT_SIZE, ANT_SIZE, 0.0),
                ..Default::default()
            },
            sprite: Sprite {
                custom_size: Some(Vec2::new(1.0, 1.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2))
        .insert(Ant {
            ..Default::default()
        })
        .id();

    // bottom wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(180.0, 0.1).into(),
            material: ColliderMaterial {
                friction: 0.0,
                ..Default::default()
            }
            .into(),
            position: (Vec2::new(0.0, -60.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
    // top wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(180.0, 0.1).into(),
            material: ColliderMaterial {
                friction: 0.0,
                ..Default::default()
            }
            .into(),
            position: (Vec2::new(0.0, 60.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
    // left wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(0.1, 120.0).into(),
            material: ColliderMaterial {
                friction: 0.0,
                ..Default::default()
            }
            .into(),
            position: (Vec2::new(-90.0, 0.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
    // right wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(0.1, 120.0).into(),
            material: ColliderMaterial {
                friction: 0.0,
                ..Default::default()
            }
            .into(),
            position: (Vec2::new(90.0, 0.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));

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
    collider_query: Query<(Entity, &Collider, &Transform)>,
    home_query: Query<(Entity, &Home, &Transform)>,
    food_query: Query<(Entity, &Food, &Transform)>,
    icon_query: Query<(&Icon, &Transform)>,
    map_transform_query: Query<&Transform, With<Map>>,
    mut map_query: MapQuery,
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
                        if !collider_query.iter().any(|(_, _, transform)| {
                            pos_in_transform(&world_cursor_pos, &transform)
                        }) {
                            let tile_pos = tile_pos_from_world_pos(
                                &world_cursor_pos,
                                &mut map_query,
                                map_transform_query.single(),
                                0,
                                0,
                            );
                            spawn_obstacle(tile_pos, &mut commands, &mut map_query);
                        }
                    } else if buttons.pressed(MouseButton::Right) {
                        // TODO: fix obstacle removal
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
    asset_server: Res<AssetServer>,
    mut map_generator: ResMut<MapGenerator>,
    mut map_query: MapQuery,
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

    if map_query.get_layer(0, 0).is_none() {
        let texture_handle = asset_server.load("tiles_10.png");

        // Create map entity and component:
        let map_entity = commands.spawn().id();
        let mut map = Map::new(0u16, map_entity);

        let layer_settings = LayerSettings::new(
            MapSize((BOUNDS_X / 100.0) as u32, (BOUNDS_Y / 100.0) as u32),
            ChunkSize(10, 10),
            TileSize(10.0, 10.0),
            TextureSize(60.0, 10.0),
        );

        // Creates a new layer builder with a layer entity.
        let (mut layer_builder, _) =
            LayerBuilder::<TileBundle>::new(&mut commands, layer_settings, 0u16, 0u16);

        for tile_bundle in generate_map_tiles(map_generator) {
            let _ = layer_builder.set_tile(tile_bundle.position, tile_bundle);
        }

        // Builds the layer.
        // Note: Once this is called you can no longer edit the layer until a hard sync in bevy.
        let layer_entity = map_query.build_layer(&mut commands, layer_builder, texture_handle);

        // Required to keep track of layers for a map internally.
        map.add_layer(&mut commands, 0u16, layer_entity);

        // Spawn Map
        // Required in order to use map_query to retrieve layers/tiles.
        commands
            .entity(map_entity)
            .insert(map)
            .insert(Transform::from_xyz(
                -layer_settings.get_pixel_center().x,
                -layer_settings.get_pixel_center().y,
                0.0,
            ))
            .insert(GlobalTransform::default());
    } else {
        despawn_layer_tiles_and_notify_chunks(&mut commands, &mut map_query, 0, 0);
        // Generate a new set of obstacles
        for tile_bundle in generate_map_tiles(map_generator) {
            let _result = map_query.set_tile(
                &mut commands,
                tile_bundle.position,
                tile_bundle.tile,
                0u16,
                0u16,
            );
            map_query.notify_chunk_for_tile(tile_bundle.position, 0u16, 0u16);
        }
    }
}

fn generate_map_tiles(map_generator: ResMut<MapGenerator>) -> Vec<TileBundle> {
    let mut tile_bundles = Vec::new();
    let noise = HybridMulti::new()
        .set_octaves(map_generator.octaves)
        .set_frequency(map_generator.frequency)
        .set_lacunarity(map_generator.lacunarity)
        .set_persistence(map_generator.persistence);
    let num_tiles_x = (BOUNDS_X / OBSTACLE_TILE_SIZE) as i32;
    let num_tiles_y = (BOUNDS_Y / OBSTACLE_TILE_SIZE) as i32;
    for i in 0..num_tiles_x {
        for j in 0..num_tiles_y {
            let ox = OBSTACLE_TILE_SIZE * ((i - num_tiles_x / 2) as f32) + OBSTACLE_TILE_SIZE / 2.0;
            let oy = OBSTACLE_TILE_SIZE * ((num_tiles_y / 2 - j) as f32) - OBSTACLE_TILE_SIZE / 2.0;
            if noise.get([ox as f64, oy as f64]) < map_generator.threshold {
                continue;
            }
            tile_bundles.push(TileBundle {
                position: TilePos(i as u32, (num_tiles_y - j) as u32),
                tile: Tile {
                    texture_index: 0,
                    ..Default::default()
                },
                ..Default::default()
            });
        }
    }
    tile_bundles
}

fn spawn_obstacle(tile_pos: TilePos, commands: &mut Commands, map_query: &mut MapQuery) {
    let _result = map_query.set_tile(
        commands,
        tile_pos,
        Tile {
            texture_index: 0,
            ..Default::default()
        },
        0u16,
        0u16,
    );
    map_query.notify_chunk_for_tile(tile_pos, 0u16, 0u16);
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
    for _ in 0..40 {
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
    mut map_query: MapQuery,
    map_transform_query: Query<&Transform, (With<Map>, Without<Ant>)>,
) {
    for (_ant, mut ant_transform) in ant_query.iter_mut() {
        let ant_size = ant_transform.scale.truncate();

        // check collision with walls
        // TODO: converge walls and tiles
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

                    // only reflect if the ant's velocity is going in the opposite direction of the
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
        let collisions = collide_tiles_with_rect(
            ant_transform.translation,
            ant_size,
            &mut map_query,
            map_transform_query.single(),
            0,
            0,
        );
        for collision in collisions {
            // reflect the ball when it collides
            let mut reflect_x = false;
            let mut reflect_y = false;
            let direction = ant_transform.rotation * Vec3::X;

            // only reflect if the ant's velocity is going in the opposite direction of the
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

fn ant_movement_system2(
    keys: Res<Input<KeyCode>>,
    mut rigid_bodies: Query<(
        &Ant,
        &mut RigidBodyForcesComponent,
        &mut RigidBodyVelocityComponent,
        &RigidBodyMassPropsComponent,
        &mut RigidBodyPositionComponent,
    )>,
) {
    for (ant, mut rb_forces, rb_vel, _rb_mprops, rb_pos) in rigid_bodies.iter_mut() {
        // Motor forces
        let object_x_axis = rb_pos.position.rotation * Vector2::x_axis();
        let object_x_velocity = rb_vel.linvel.dot(&object_x_axis) * object_x_axis.into_inner();
        if !keys.pressed(KeyCode::Down) {
            rb_forces.force += rb_pos.position.rotation
                * Vector2::x_axis().into_inner()
                * (ant.target_speed - object_x_velocity.norm())
                * ant.motor_force;
        }

        // Grip forces
        let object_y_axis = rb_pos.position.rotation * Vector2::y_axis();
        let object_y_velocity = rb_vel.linvel.dot(&object_y_axis) * object_y_axis.into_inner();
        rb_forces.force -= object_y_velocity * ant.grip_force;

        // Turning input
        if keys.pressed(KeyCode::Left) {
            rb_forces.torque += ant.turning_torque;
        }
        if keys.pressed(KeyCode::Right) {
            rb_forces.torque -= ant.turning_torque;
        }

        // Random wandering
        rb_forces.torque += ant.random_turning_torque * (random::<f32>() * 2.0 - 1.0);
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
        let wandering_angle_delta =
            config.entries["ant.wandering"].f32() * (random::<f32>() * 2.0 - 1.0);

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

pub fn set_texture_filters_to_nearest(
    mut texture_events: EventReader<AssetEvent<Image>>,
    mut textures: ResMut<Assets<Image>>,
) {
    // quick and dirty, run this for all textures anytime a texture is created.
    for event in texture_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                if let Some(mut texture) = textures.get_mut(handle) {
                    texture.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST;
                }
            }
            _ => (),
        }
    }
}
