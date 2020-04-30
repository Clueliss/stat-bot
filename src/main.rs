#[macro_use]
extern crate lazy_static;

extern crate serenity;

use serenity::client::Client;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Guild;
use serenity::model::id::{GuildId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::{EventHandler, Context};

use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::sync::{Mutex, LockResult, MutexGuard, Arc};


lazy_static! {
    static ref ONLINE_TIME: Mutex<HashMap<UserId, Duration>> = Mutex::new(HashMap::new());
    static ref ONLINE_SINCE: Mutex<HashMap<UserId, SystemTime>> = Mutex::new(HashMap::new());
}

fn seconds_to_human_readable(s_total: u64) -> String {
    let d = s_total/86400;
    let h = (s_total - d * 86400)/3600;
    let m = ((s_total - d * 86400) - h * 3600)/60;
    let s = ((s_total - d * 86400) - h * 3600) - (m * 60);

    format!("*{}* ***D***, *{}* ***H***, *{}* ***M***, *{}* ***S***", d, h, m, s)
}


fn stat_json(ctx: &Context) -> String {
    let ontime = ONLINE_TIME.lock().unwrap();
    let mut buf = "{\n".to_string();

    for (uid, time) in ontime.iter() {
        buf += &format!("    \"{}\": \"{}\"\n",
                        uid.to_user(ctx).unwrap().name,
                        time.as_secs());
    }

    buf + "}"
}


fn stat_human_readable(ctx: &Context) -> String {
    let ontime = ONLINE_TIME.lock().unwrap();
    let mut buf = String::new();

    for (uid, time) in ontime.iter() {
        buf += &format!("{}: {}\n",
                        uid.to_user(ctx).unwrap().name,
                        seconds_to_human_readable(time.as_secs()));
    }

    buf
}


fn update_stats() {
    let mut onsince = ONLINE_SINCE.lock().unwrap();
    let mut ontime = ONLINE_TIME.lock().unwrap();

    for (uid, timestamp) in onsince.iter_mut() {
        let duration = SystemTime::now()
            .duration_since(timestamp.clone())
            .unwrap();

        match ontime.get_mut(&uid) {
            Some(t) => { *t += duration; },
            None    => { ontime.insert(uid.clone(), duration); }
        }

        *timestamp = SystemTime::now();
    }
}


struct StatBot;

impl EventHandler for StatBot {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.content == ">>stats" {
            update_stats();

            msg.channel_id
                .send_message(&ctx, |m| m.content(stat_human_readable(&ctx)))
                .unwrap();
        }
    }

    fn ready(&self, _ctx: Context, _rdy: Ready) {
        println!("<6> now online");
    }

    fn voice_state_update(&self, ctx: Context, _: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {

        let mut ontime = ONLINE_TIME.lock().unwrap();
        let mut onsince = ONLINE_SINCE.lock().unwrap();

        if old.map(|o| o.channel_id) != Some(new.channel_id) {
            match new.channel_id {
                None => {
                    println!("User left: {}", new.user_id.to_user(&ctx).unwrap().name);

                    if onsince.contains_key(&new.user_id) {
                        let since = onsince.remove(&new.user_id).unwrap();

                        let duration = SystemTime::now()
                            .duration_since(since)
                            .unwrap();

                        match ontime.get_mut(&new.user_id) {
                            Some(time) => { *time += duration; },
                            None       => { ontime.insert(new.user_id, duration); },
                        }
                    }
                },
                Some(_) => {
                    println!("User joined: {}", new.user_id.to_user(ctx).unwrap().name);

                    if !onsince.contains_key(&new.user_id) {
                        onsince.insert(new.user_id, SystemTime::now());
                    }
                }
            }
        }

        println!("onsince: {:?}", onsince);
        println!("ontime: {:?}", ontime);
    }

}

fn main() {
    let tok = std::env::var("STAT_BOT_DISCORD_TOKEN").unwrap();
    let mut client = Client::new(tok, StatBot).unwrap();

    client.start().unwrap();
}
