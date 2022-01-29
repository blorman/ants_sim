use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use nalgebra::{Point2, Vector2};
use rand::random;

const ANT_SIZE: f32 = 5.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierRenderPlugin)
        .insert_resource(RapierConfiguration {
            scale: 5.0,
            gravity: Vector::new(0.0, 0.0),
            ..Default::default()
        })
        .add_startup_system(setup.label("setup"))
        .add_system(ant_movement_system)
        .run()
}

fn ant_movement_system(
    keys: Res<Input<KeyCode>>,
    mut rigid_bodies: Query<
        (
            &mut RigidBodyForcesComponent,
            &mut RigidBodyVelocityComponent,
            &RigidBodyMassPropsComponent,
            &mut RigidBodyPositionComponent,
        ),
        With<Ant>,
    >,
) {
    for (mut rb_forces, rb_vel, _rb_mprops, rb_pos) in rigid_bodies.iter_mut() {
        let motor_force = 4.0;
        let grip_force = 5.0;
        let turning_torque = 2.0;
        let random_turning_torque = 5.0;

        // Motor forces
        let object_x_axis = rb_pos.position.rotation * Vector2::x_axis();
        let object_x_velocity = rb_vel.linvel.dot(&object_x_axis) * object_x_axis.into_inner();
        if !keys.pressed(KeyCode::Down) {
            rb_forces.force += rb_pos.position.rotation
                * Vector2::new(2.0, 0.0)
                * (8.0 - object_x_velocity.norm())
                * motor_force;
        }

        // Grip forces
        let object_y_axis = rb_pos.position.rotation * Vector2::y_axis();
        let object_y_velocity = rb_vel.linvel.dot(&object_y_axis) * object_y_axis.into_inner();
        rb_forces.force -= object_y_velocity * grip_force;

        // Turning input
        if keys.pressed(KeyCode::Left) {
            rb_forces.torque += turning_torque;
        }
        if keys.pressed(KeyCode::Right) {
            rb_forces.torque -= turning_torque;
        }

        // Random wandering
        rb_forces.torque += random_turning_torque * (random::<f32>() * 2.0 - 1.0);
    }
}

#[derive(Component)]
struct Ant {}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scale = 0.25;
    commands.spawn_bundle(camera);

    /* Create the bouncing ball. */
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
        .insert(Ant {})
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
            position: (Vec2::new(0.0, -15.0)).into(),
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
            position: (Vec2::new(0.0, 15.0)).into(),
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
            position: (Vec2::new(-20.0, 0.0)).into(),
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
            position: (Vec2::new(20.0, 0.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
}
