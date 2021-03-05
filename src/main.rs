extern crate chrono;
extern crate serde_json;
extern crate serenity;
extern crate tempfile;
extern crate signal_hook;

mod stats;
mod stat_bot;
mod graphing;

use clap::Clap;
use serenity::client::Client;
use std::fs::File;
use stat_bot::Settings;
use std::sync::{Arc, Mutex};
use crate::stats::StatManager;


#[derive(Clap)]
struct Opts {
    #[clap(short = 's', long = "settings-file")]
    settings_file: String,
}

fn main() {
    let opts: Opts = Opts::parse();

    let settings: Settings = match File::open(&opts.settings_file) {
            Ok(f) => serde_json::from_reader(f).expect("invalid json in config"),
            Err(_) => Settings::default(),
        };

    let stat_man = Arc::new(Mutex::new({
        let mut s = StatManager::new(&settings.output_dir);
        s.read_stats()
            .expect("failed to read stats");

        s
    }));

    let tok = std::env::var("STAT_BOT_DISCORD_TOKEN")
        .expect("failed to read token from env");

    let mut client = Client::new(tok, stat_bot::StatBot::new(&opts.settings_file, settings, stat_man.clone()))
        .expect("failed to create discord client");

    unsafe {
        signal_hook::register(signal_hook::SIGINT, move || {
            stat_man.lock().expect("lock failed in sighandler")
                .flush_stats()
                .expect("could not flush stats");
        }).expect("failed to register signal handler");
    }

    client.start().unwrap();
}
