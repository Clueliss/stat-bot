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


#[derive(Clap)]
struct Opts {
    #[clap(short = "o", long = "outdir")]
    outputdir: String
}


fn signal_handler(sig: libc::c_int) {
    let outdir = stat_bot::OUTPUT_DIR.lock().unwrap();
    let mut st = stat_bot::STATS.lock().unwrap();

    let f = File::create(&format!("{}/{}", &*outdir, stat_bot::STAT_FILE_NAME)).unwrap();
    st.flush_stats(f).unwrap();

    {
        let trans = st.generate_translations();
        let trans_file = File::create(&format!("{}/{}", &*outdir, stat_bot::TRANS_FILE_NAME)).unwrap();
        serde_json::to_writer(trans_file, &trans).unwrap();
    }

    if sig == libc::SIGTERM {
        std::process::exit(0);
    }
}


fn main() {
    let opts: Opts = Opts::parse();

    unsafe {
        let signal_handler_fn_ptr = signal_handler as *const fn(libc::c_int);
        let sighandler = std::mem::transmute::<*const fn(libc::c_int), libc::sighandler_t>(signal_handler_fn_ptr);

        libc::signal(libc::SIGTERM, sighandler);
        libc::signal(libc::SIGINT, sighandler);
    }

    let settings = {
        let outdir = stat_bot::OUTPUT_DIR.lock().unwrap();

        File::open(format!("{}/{}", outdir, stat_bot::SETTINGS_FILE_NAME))
            .and_then(|f| stat_bot::Settings::load(f).map_err(Into::into))
            .unwrap_or_default()
    };

    let tok = std::env::var("STAT_BOT_DISCORD_TOKEN").unwrap();
    let mut client = Client::new(tok, stat_bot::StatBot::new(settings)).unwrap();

    {
        let mut st = stat_bot::STATS.lock().unwrap();

        match File::open(&format!("{}/{}", &opts.outputdir, stat_bot::STAT_FILE_NAME)) {
            Ok(f) => st.read_stats(f).unwrap(),
            _ => (),
        }

        *stat_bot::OUTPUT_DIR.lock().unwrap() = opts.outputdir;

        st.set_cache_and_http(client.cache_and_http.clone());
    }

    client.start().unwrap();
}
