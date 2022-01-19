mod ants_plugin;
mod console_debug_plugin;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(console_debug_plugin::ConsoleDebugPlugin)
        .add_plugin(ants_plugin::AntsPlugin)
        .run()
}
