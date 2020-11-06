extern crate chrono;
extern crate libc;
extern crate serde_json;
extern crate serenity;
extern crate tempfile;
extern crate signal_hook;

mod stats;
mod stat_bot;


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

    #[clap(short= 'g', long = "graphing-tool-path")]
    graphing_tool_path: String,
}

fn main() {
    let opts: Opts = Opts::parse();

    let settings: Settings = match File::open(&opts.settings_file) {
            Ok(f) => serde_json::from_reader(f).unwrap(),
            Err(_) => Settings::default(),
        };

    let stat_man = Arc::new(Mutex::new({
        let mut s = StatManager::default();
        s.set_output_dir(&settings.output_dir);
        s.set_graphing_tool_path(&opts.graphing_tool_path);

        s
    }));

    let tok = std::env::var("STAT_BOT_DISCORD_TOKEN").unwrap();
    let mut client = Client::new(tok, stat_bot::StatBot::new(&opts.settings_file, settings.clone(), stat_man.clone())).unwrap();

    stat_man.lock().unwrap().set_cache_and_http(client.cache_and_http.clone());

    unsafe {
        signal_hook::register(signal_hook::SIGINT, move || {
            stat_man.lock().unwrap().flush_stats().unwrap();
        }).unwrap();
    }

    client.start().unwrap();
}
