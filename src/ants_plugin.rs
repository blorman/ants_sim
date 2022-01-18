use bevy::prelude::*;

#[derive(Component)]
struct Ant;

#[derive(Component)]
struct Name(String);

struct GreetTimer(Timer);

fn add_ants(mut commands: Commands) {
    commands.spawn().insert(Ant).insert(Name("Bob".to_string()));
    commands.spawn().insert(Ant).insert(Name("Pam".to_string()));
    commands.spawn().insert(Ant).insert(Name("Max".to_string()));
}

fn greet_ants(time: Res<Time>, mut timer: ResMut<GreetTimer>, query: Query<&Name, With<Ant>>) {
    if timer.0.tick(time.delta()).just_finished() {
        for name in query.iter() {
            println!("hello {}!", name.0);
        }
    }
}

pub struct AntsPlugin;

impl Plugin for AntsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GreetTimer(Timer::from_seconds(2.0, true)))
            .add_startup_system(add_ants)
            .add_system(greet_ants);
    }
}
