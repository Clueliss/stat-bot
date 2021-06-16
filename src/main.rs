#![feature(slice_group_by)]

extern crate chrono;
extern crate serenity;
extern crate signal_hook;
extern crate tempfile;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

mod graphing;
mod model;
mod schema;
mod stat_bot;
mod stats;

use diesel::{Connection, PgConnection};
use stats::StatManager;
use clap::Clap;
use serenity::client::Client;
use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};
use stat_bot::Settings;
use std::fs::File;
use std::sync::{Arc, RwLock};

embed_migrations!("migrations");

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

    let tok = std::env::var("STAT_BOT_DISCORD_TOKEN").expect("failed to read token from env");
    let dburl = std::env::var("DATABASE_URL").expect("database url not found in env");

    embedded_migrations::run(&PgConnection::establish(&dburl).expect("unable to establish db connection"))
        .expect("unable to run migrations");

    let stat_man = Arc::new(RwLock::new(StatManager::new(&dburl)));
    let timer_stat_man = stat_man.clone();
    let signal_stat_man = stat_man.clone();

    let mut client = Client::new(
        tok,
        stat_bot::StatBot::new(&opts.settings_file, settings, stat_man),
    )
    .expect("failed to create discord client");

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60 * 60));
        timer_stat_man.write().unwrap().flush_stats().unwrap();
    });

    let mut signals = Signals::new(&[SIGTERM, SIGQUIT, SIGINT]).unwrap();
    std::thread::spawn(move || {
        for _ in signals.forever() {
            if let Err(e) = signal_stat_man.write().unwrap().flush_stats() {
                eprintln!("Error flushing stats on term: {}", e);
            }

            std::process::exit(0);
        }
    });

    client.start().unwrap();
}
