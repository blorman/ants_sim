use bevy::{
    core::FixedTimestep,
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};
use rand::prelude::random;

pub struct AntsPlugin;

const TIME_STEP: f32 = 1.0 / 60.0;

impl Plugin for AntsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup).add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(ant_collision_system)
                .with_system(ant_movement_system),
        );
    }
}

#[derive(Component)]
struct Ant {
    velocity: Vec3,
}

#[derive(Component)]
struct Name(String);

#[derive(Component)]
enum Collider {
    Solid,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("ant.png"),
            transform: Transform {
                scale: Vec3::new(5.0, 5.0, 0.0),
                translation: Vec3::new(0.0, -50.0, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                custom_size: Some(Vec2::new(1.0, 1.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Ant {
            velocity: 50.0
                * Vec3::new(random::<f32>() - 0.5, random::<f32>() - 0.5, 0.0).normalize(),
        })
        .insert(Name("Bob".to_string()));

    // Add walls
    let wall_color = Color::rgb(0.8, 0.8, 0.8);
    let wall_thickness = 10.0;
    let bounds = Vec2::new(900.0, 600.0);

    // left
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-bounds.x / 2.0, 0.0, 0.0),
                scale: Vec3::new(wall_thickness, bounds.y + wall_thickness, 1.0),
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
                translation: Vec3::new(bounds.x / 2.0, 0.0, 0.0),
                scale: Vec3::new(wall_thickness, bounds.y + wall_thickness, 1.0),
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
                translation: Vec3::new(0.0, -bounds.y / 2.0, 0.0),
                scale: Vec3::new(bounds.x + wall_thickness, wall_thickness, 1.0),
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
                translation: Vec3::new(0.0, bounds.y / 2.0, 0.0),
                scale: Vec3::new(bounds.x + wall_thickness, wall_thickness, 1.0),
                ..Default::default()
            },
            sprite: Sprite {
                color: wall_color,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid);
}

fn ant_collision_system(
    mut ant_query: Query<(&mut Ant, &Transform)>,
    collider_query: Query<(&Collider, &Transform)>,
) {
    let (mut ant, ant_transform) = ant_query.single_mut();
    let ant_size = ant_transform.scale.truncate();
    let velocity = &mut ant.velocity;

    // check collision with walls
    for (_collider, transform) in collider_query.iter() {
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

            // only reflect if the ball's velocity is going in the opposite direction of the
            // collision
            match collision {
                Collision::Left => reflect_x = velocity.x > 0.0,
                Collision::Right => reflect_x = velocity.x < 0.0,
                Collision::Top => reflect_y = velocity.y < 0.0,
                Collision::Bottom => reflect_y = velocity.y > 0.0,
            }

            // reflect velocity on the x-axis if we hit something on the x-axis
            if reflect_x {
                velocity.x = -velocity.x;
            }

            // reflect velocity on the y-axis if we hit something on the y-axis
            if reflect_y {
                velocity.y = -velocity.y;
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

fn ant_movement_system(mut query: Query<(&Ant, &mut Transform)>) {
    let (ant, mut transform) = query.single_mut();
    transform.translation += ant.velocity * TIME_STEP;
    let angle = vec3_angle(ant.velocity) - std::f32::consts::PI / 4.0;
    transform.rotation = Quat::from_rotation_z(angle);
}
