mod ants_plugin;
// mod console_debug_plugin;
mod helpers;

// use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugin(LogDiagnosticsPlugin::default())
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(console_debug_plugin::ConsoleDebugPlugin)
        .add_plugin(ants_plugin::AntsPlugin)
        .run()
}
