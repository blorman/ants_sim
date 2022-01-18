mod ants_plugin;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(ants_plugin::AntsPlugin)
        .run()
}
