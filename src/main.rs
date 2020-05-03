extern crate chrono;
#[macro_use] extern crate lazy_static;
extern crate libc;
extern crate serde_json;
extern crate serenity;

mod stats;


use clap::Clap;

use serenity::client::Client;
use serenity::model::channel::{Message, GuildChannel, ChannelType};
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, ChannelId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::{EventHandler, Context};

use stats::Stats;

use std::collections::HashMap;
use std::fs::File;
use std::sync::Mutex;


static STAT_FILE_NAME: &str = "stat.json";
static TRANS_FILE_NAME: &str = "trans.json";

lazy_static! {
    static ref STATS: Mutex<Stats> = Mutex::new(Stats::new());
    static ref OUTPUT_DIR: Mutex<String> = Mutex::new(String::new());
}


#[derive(Clap)]
struct Opts {
    #[clap(short = "o", long = "outdir")]
    outputdir: String
}

struct StatBot;

impl EventHandler for StatBot {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.content == ">>stats" {
            let mut st = STATS.lock().unwrap();
            st.update_stats();

            msg.channel_id
                .send_message(&ctx, |m| m.content(st.as_human_readable_string()))
                .unwrap();
        }
    }

    fn ready(&self, ctx: Context, rdy: Ready) {
        let mut st = STATS.lock().unwrap();
        println!("now online");
        let tlof = rdy.guilds.get(0).unwrap();

        let channels: HashMap<ChannelId, GuildChannel> = tlof.id().channels(&ctx).unwrap();

        for (_id, ch) in channels {
            match ch.kind {
                ChannelType::Voice if !ch.name.starts_with("AFK") => {
                    let members: Vec<Member> = ch.members(&ctx).unwrap();

                    for m in members {
                        let u: User = m.user_id().to_user(&ctx).unwrap();

                        if !u.bot {
                            st.user_now_online(u.id);
                        }
                    }
                },
                _ => (),
            }
        }
    }

    fn voice_state_update(&self, ctx: Context, _: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
        let mut st = STATS.lock().unwrap();

        if old.map(|o| o.channel_id) != Some(new.channel_id) {
            match new.channel_id {
                Some(id) if !id.name(&ctx).unwrap().starts_with("AFK") => {
                    st.user_now_online(new.user_id);
                    println!("User joined: {}", new.user_id.to_user(ctx).unwrap().name);
                },
                _ => {
                    st.user_now_offline(new.user_id);
                    println!("User left: {}", new.user_id.to_user(&ctx).unwrap().name);
                },
            }
        }
    }

}


fn signal_handler(sig: libc::c_int) {
    let outdir = OUTPUT_DIR.lock().unwrap();
    let mut st = STATS.lock().unwrap();

    let mut f = File::create(&format!("{}/{}", &*outdir, STAT_FILE_NAME)).unwrap();
    st.flush_stats(&mut f).unwrap();

    {
        let trans = st.generate_translations();
        let mut trans_file = File::create(&format!("{}/{}", &*outdir, TRANS_FILE_NAME)).unwrap();
        serde_json::to_writer(&mut trans_file, &trans).unwrap();
    }

    if sig == libc::SIGTERM {
        std::process::exit(0);
    }
}


fn main() {
    unsafe {
        let signal_handler_fn_ptr = signal_handler as *const fn(libc::c_int);
        let sighandler = std::mem::transmute::<*const fn(libc::c_int), libc::sighandler_t>(signal_handler_fn_ptr);

        libc::signal(libc::SIGTERM, sighandler);
        libc::signal(libc::SIGINT, sighandler);
    }

    let tok = std::env::var("STAT_BOT_DISCORD_TOKEN").unwrap();
    let mut client = Client::new(tok, StatBot).unwrap();

    {
        let mut st = STATS.lock().unwrap();

        let opts: Opts = Opts::parse();

        match File::open(&format!("{}/{}", &opts.outputdir, STAT_FILE_NAME)) {
            Ok(mut f) => st.read_stats(&mut f).unwrap(),
            _ => (),
        }

        *OUTPUT_DIR.lock().unwrap() = opts.outputdir;

        st.set_cache_and_http(client.cache_and_http.clone());
    }

    client.start().unwrap();
}
