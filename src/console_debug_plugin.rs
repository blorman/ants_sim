use bevy::{
    prelude::*,
    tasks::AsyncComputeTaskPool,
};
use crossbeam::channel::{bounded, Receiver};
use clap::{App, ArgMatches};
use std::io::{self, BufRead, Write};

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
    line_channel: Res<Receiver<String>>,
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

        let output = match_commands(&matches);

        println!("{}", output);
        print!(">>> ");
        io::stdout().flush().unwrap();
    }
}

pub fn build_commands<'a>(app_name: &'a str) -> App {
    let app = clap::App::new(app_name)
        .subcommand(clap::App::new("foo").about("foo bar"));
    app
}

pub fn match_commands(matches: &ArgMatches) -> String {
        let mut output = String::new();
    match matches.subcommand() {
        Some(("foo", _)) => {
            output.push_str("...foo command!.");
        }
        _ => {}
    }
    output
}

pub struct ConsoleDebugPlugin;
impl Plugin for ConsoleDebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(spawn_io_thread).add_system(parse_input.system());
    }
}