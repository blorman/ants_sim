use bevy::{
    prelude::*,
    tasks::AsyncComputeTaskPool,
};
use crossbeam::channel::{bounded, Receiver};
use clap::{App, ArgMatches};
use std::io::{self, BufRead, Write};
use std::collections::HashMap;

fn spawn_io_thread(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    println!("Bevy Console Debugger.  Type 'help' for list of commands.");
    print!(">>> ");
    io::stdout().flush().unwrap();

    let (tx, rx) = bounded(1);
    let task = thread_pool.spawn(async move {
        let stdin = io::stdin();
        loop {
            let line = stdin.lock().lines().next().unwrap().unwrap();
            tx.send(line)
                .expect("error sending user input to other thread");
        }
    });
    task.detach();
    commands.insert_resource(rx);
}

fn parse_input(
    line_channel: Res<Receiver<String>>, mut config: ResMut<Config>
) {
    if let Ok(line) = line_channel.try_recv() {
        let app_name = "";
        println!("" );
        let split = line.split_whitespace();
        let mut args = vec![app_name];
        args.append(&mut split.collect());

        let matches_result = build_commands(app_name).try_get_matches_from(args);

        if let Err(e) = matches_result {
            println!("{}", e.to_string());
            print!(">>> ");
            io::stdout().flush().unwrap();
            return;
        }

        let matches = matches_result.unwrap();

        let output = match_commands(&matches, &mut config);

        println!("{}", output);
        print!(">>> ");
        io::stdout().flush().unwrap();
    }
}

pub enum ConfigValue {
    Int(i32),
    Float(f32),
    String(String)
}

impl ConfigValue {
    pub fn f32(&self) -> f32 {
        match self {
            ConfigValue::Float(f) => *f,
            _ => 0.0
        }
    }
    pub fn f64(&self) -> f64 {
        match self {
            ConfigValue::Float(f) => *f as f64,
            _ => 0.0
        }
    }
    pub fn usize(&self) -> usize {
        match self {
            ConfigValue::Int(i) => *i as usize,
            _ => 0
        }
    }
}

#[derive(Default)]
pub struct Config {
    pub entries: HashMap<&'static str, ConfigValue>,
}

pub fn build_commands<'a>(app_name: &'a str) -> App {
    let app = clap::App::new(app_name)
        .subcommand(clap::App::new("config_get")
            .about("get convig value")
            .arg(clap::arg!([key] "'string key of entry to get'")))
        .subcommand(clap::App::new("config_set")
            .about("set convig value")
            .arg(clap::arg!([key] "'string key of entry to set'"))
            .arg(clap::arg!([value] "'value of entry to set'")));
    app
}

pub fn match_commands(matches: &ArgMatches, config: &mut Config) -> String {
        let mut output = String::new();
    match matches.subcommand() {
        Some(("foo", _)) => {
            output.push_str("...foo command!.");
        }
        Some(("config_get", s_matches)) => {
            output.push_str("...config_get command!");
            if let Some(key) = s_matches.value_of("key") {
              output.push_str(" key: ");
              output.push_str(key);
              output.push_str(" value: ");
              if let Some(value) = config.entries.get(key) {
                  match value {
                    ConfigValue::Int(i) => output.push_str(&i.to_string()[..]),
                    ConfigValue::Float(f) => output.push_str(&f.to_string()[..]),
                    ConfigValue::String(s) => output.push_str(&s[..]),
                  }
              }
            }
        }
        Some(("config_set", s_matches)) => {
            output.push_str("...config_set command!.");
            if let Some(key) = s_matches.value_of("key") {
              // let key: &'static str = key;
              output.push_str(" key: ");
              output.push_str(key);
              if let Some(new_value) = s_matches.value_of("value") {
                let entries = &mut config.entries;
                  if let Some((ref mut old_key, old_value)) = entries.get_key_value(key) {
                      let foo = match old_value {
                        ConfigValue::Int(_) => {ConfigValue::Int(new_value.parse::<i32>().unwrap())},
                        ConfigValue::Float(_) => {ConfigValue::Float(new_value.parse::<f32>().unwrap())},
                        ConfigValue::String(_) => {ConfigValue::String(new_value.to_string())},
                      };
                      entries.insert(old_key, foo);
                  }
              }
            }
        }
        _ => {}
    }
    output
}

pub struct ConsoleDebugPlugin;
impl Plugin for ConsoleDebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<Config>().add_startup_system(spawn_io_thread).add_system(parse_input.system());
    }
}