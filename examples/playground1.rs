use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

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
        .run()
}

#[derive(Component)]
struct AntController {}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scale = 1.0;
    commands.spawn_bundle(camera);

    /* Create the bouncing ball. */
    let rigid_body = RigidBodyBundle {
        position: Vec2::new(0.0, 1.0).into(),
        damping: RigidBodyDamping {
            linear_damping: 0.1,
            angular_damping: 0.1,
        }
        .into(),
        ..Default::default()
    };
    let collider = ColliderBundle {
        shape: ColliderShape::cuboid(0.5, 0.25).into(),
        material: ColliderMaterial {
            restitution: 0.7,
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };
    let entity1 = commands
        .spawn_bundle(rigid_body)
        .insert_bundle(collider)
        .insert_bundle(SpriteBundle {
            texture: asset_server.load("ant.png"),
            transform: Transform {
                scale: Vec3::new(ANT_SIZE, ANT_SIZE, 0.0),
                // translation: Vec3::new(0.0, -50.0, 0.0),
                // rotation: Quat::from_rotation_z(random::<f32>() * 2.0 * std::f32::consts::PI),
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
        .id();

    let entity2 = commands
        .spawn_bundle(RigidBodyBundle {
            body_type: RigidBodyType::Static.into(),
            ..Default::default()
        })
        .insert(AntController {})
        .id();

    let joint = PrismaticJoint::new(Vector::x_axis())
        .local_anchor1(point![0.0, 1.0])
        .local_anchor2(point![0.0, 0.0])
        .motor_velocity(8.0, 1.0);
    commands
        .spawn()
        .insert(JointBuilderComponent::new(joint, entity1, entity2));

    // bottom wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(180.0, 0.1).into(),
            position: (Vec2::new(0.0, -60.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
    // top wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(180.0, 0.1).into(),
            position: (Vec2::new(0.0, 60.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
    // left wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(0.1, 120.0).into(),
            position: (Vec2::new(-90.0, 0.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
    // right wall
    commands
        .spawn_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(0.1, 120.0).into(),
            position: (Vec2::new(90.0, 0.0)).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::with_id(2));
}
