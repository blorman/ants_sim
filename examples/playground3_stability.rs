use bevy::prelude::*;
use bevy_rapier2d::physics::TimestepMode;
use bevy_rapier2d::prelude::*;
use nalgebra::Vector2;
use rand::random;
use rand::rngs::StdRng;

use rand::Rng;

use rand::SeedableRng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierRenderPlugin)
        .insert_resource(RapierConfiguration {
            scale: 5.0,
            gravity: Vector::new(0.0, 0.0),
            timestep_mode: TimestepMode::FixedTimestep,
            ..Default::default()
        })
        .add_startup_system(setup.label("setup"))
        .add_system(ant_movement_system)
        .run()
}

#[derive(Component)]
struct Ant {
    target_speed: f32,
    motor_force: f32,
    grip_force: f32,
    turning_torque: f32,
    random_turning_torque: f32,
    rng_can: Vec<f32>,
    index: usize,
}

impl Default for Ant {
    fn default() -> Ant {
        Ant {
            target_speed: 8.0,
            motor_force: 4.0,
            grip_force: 5.0,
            turning_torque: 2.0,
            random_turning_torque: 5.0,
            rng_can: Vec::new(),
            index: 0,
        }
    }
}

fn ant_movement_system(
    keys: Res<Input<KeyCode>>,
    mut current_tick: Local<usize>,
    mut rigid_bodies: Query<(
        Entity,
        &Ant,
        &mut RigidBodyForcesComponent,
        &mut RigidBodyVelocityComponent,
        &RigidBodyMassPropsComponent,
        &mut RigidBodyPositionComponent,
    )>,
) {
    for (entity, ant, mut rb_forces, rb_vel, _rb_mprops, rb_pos) in rigid_bodies.iter_mut() {
        println!("Ant {} {:?}", ant.index, entity);
        println!(
            "  pos: {:?} angle: {}",
            rb_pos.position.translation,
            rb_pos.position.rotation.angle()
        );

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
        let random_f32 = if ant.rng_can.len() > 0 {
            ant.rng_can[*current_tick % ant.rng_can.len()]
        } else {
            random::<f32>()
        };
        rb_forces.torque += ant.random_turning_torque * (random_f32 * 2.0 - 1.0);
    }
    *current_tick += 1;
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scale = 0.25;
    commands.spawn_bundle(camera);
    let mut r = StdRng::seed_from_u64(42);

    for i in 0..2 {
        /* Create the bouncing ball. */
        let rigid_body = RigidBodyBundle {
            // position: (Vec2::new(0.0, 5.0), 5.0).into(), // Translation and rotation.
            position: if i == 0 {
                (Vec2::new(0.23, 0.324), 2.0)
            } else {
                (Vec2::new(19.0, 14.0), 0.0)
            }
            .into(),
            damping: RigidBodyDamping {
                linear_damping: 2.0,
                angular_damping: 5.0,
            }
            .into(),
            ..Default::default()
        };
        let collider = ColliderBundle {
            // TODO: debug render a capsule?
            // shape: ColliderShape::capsule(Point2::new(0.25, 0.0), Point2::new(-0.25, 0.0), 0.25)
            //     .into(),
            shape: ColliderShape::cuboid(0.5, 0.25).into(),
            material: ColliderMaterial {
                restitution: 0.7,
                friction: 0.0,
                ..Default::default()
            }
            .into(),
            ..Default::default()
        };
        let ant_entity = commands
            .spawn_bundle(rigid_body)
            .insert_bundle(collider)
            .insert(ColliderPositionSync::Discrete)
            .insert(ColliderDebugRender::with_id(1))
            .insert(Ant {
                rng_can: (0..100).map(|_i| r.gen()).collect::<Vec<_>>(),
                index: i,
                ..Default::default()
            })
            .id();

        // if i == 1 {
        //     // TODO: this seems to cause drag on the ant
        //     let collider2 = ColliderBundle {
        //         collider_type: ColliderType::Sensor.into(),
        //         shape: ColliderShape::convex_hull(&[
        //             Point2::new(0.0, 0.0),
        //             Point2::new(1.0, -1.0),
        //             Point2::new(1.0, 1.0),
        //         ])
        //         .unwrap()
        //         .into(),
        //         material: ColliderMaterial {
        //             restitution: 0.0,
        //             friction: 0.0,
        //             ..Default::default()
        //         }
        //         .into(),
        //         ..Default::default()
        //     };

        //     let collider_parent: ColliderParentComponent = ColliderParent {
        //         handle: ant_entity.handle(),
        //         pos_wrt_parent: Vec2::ZERO.into(),
        //     }
        //     .into();
        //     commands
        //         .spawn_bundle(collider2)
        //         .insert(collider_parent)
        //         .insert(ColliderPositionSync::Discrete)
        //         .insert(ColliderDebugRender::with_id(2));
        // }
    }

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
        .insert(ColliderDebugRender::with_id(3));
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
        .insert(ColliderDebugRender::with_id(3));
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
        .insert(ColliderDebugRender::with_id(3));
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
        .insert(ColliderDebugRender::with_id(3));
}
