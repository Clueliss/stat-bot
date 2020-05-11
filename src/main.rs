extern crate chrono;
#[macro_use] extern crate lazy_static;
extern crate libc;
extern crate serde_json;
extern crate serenity;

mod stats;
mod stat_bot;


use clap::Clap;
use serenity::client::Client;
use std::fs::File;
use stat_bot::Settings;


#[derive(Clap)]
struct Opts {
    #[clap(short = "s", long = "settings-file")]
    settings_file: String
}


fn signal_handler(sig: libc::c_int) {
    let mut st = stat_bot::STATS.lock().unwrap();

    st.flush_stats().unwrap();

    if sig == libc::SIGTERM {
        std::process::exit(0);
    }
}


fn main() {
    let opts: Opts = Opts::parse();

    let settings: Settings = match File::open(&opts.settings_file) {
            Ok(f) => serde_json::from_reader(f).unwrap(),
            Err(_) => Settings::default(),
        };

    unsafe {
        let signal_handler_fn_ptr = signal_handler as *const fn(libc::c_int);
        let sighandler = std::mem::transmute::<*const fn(libc::c_int), libc::sighandler_t>(signal_handler_fn_ptr);

        libc::signal(libc::SIGTERM, sighandler);
        libc::signal(libc::SIGINT, sighandler);
    }

    let tok = std::env::var("STAT_BOT_DISCORD_TOKEN").unwrap();
    let mut client = Client::new(tok, stat_bot::StatBot::new(&opts.settings_file, settings.clone())).unwrap();

    {
        let mut st = stat_bot::STATS.lock().unwrap();
        st.set_output_dir(&settings.output_dir);
        st.set_cache_and_http(client.cache_and_http.clone());

        st.read_stats().unwrap();
    }

    client.start().unwrap();
}
