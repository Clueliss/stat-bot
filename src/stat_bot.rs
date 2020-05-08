use serenity::model::channel::{Message, GuildChannel, ChannelType};
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, ChannelId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::{EventHandler, Context};

use crate::stats::Stats;

use std::collections::{HashMap, BTreeMap};
use std::fs::File;
use std::sync::Mutex;
use chrono::Utc;
use serde_json::Value;
use std::io::{Write, Read};


pub static DEFAULT_PREFIX: &str = ">>";
pub static STAT_FILE_NAME: &str = "stat.json";
pub static TRANS_FILE_NAME: &str = "trans.json";
pub static SETTINGS_FILE_NAME: &str = "settings.json";


lazy_static! {
    pub static ref STATS: Mutex<Stats> = Mutex::new(Stats::new());
    pub static ref OUTPUT_DIR: Mutex<String> = Mutex::new(String::new());
}


#[derive(Clone)]
pub struct Settings {
    pub prefix: String,
}

impl Settings {
    pub fn load<R: Read>(f: R) -> Result<Self, serde_json::Error> {
        let settings: BTreeMap<String, Value> = serde_json::from_reader(f)?;

        Ok(settings.get("prefix")
            .and_then(Value::as_str)
            .map(|p| Self{ prefix: p.to_string() })
            .unwrap_or_default())
    }

    pub fn store<W: Write>(&self, f: W) -> Result<(), serde_json::Error> {
        let conf = {
            let mut buf = BTreeMap::new();
            buf.insert("prefix", &self.prefix);

            buf
        };

        serde_json::to_writer(f, &conf)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self{ prefix: DEFAULT_PREFIX.to_string() }
    }
}


pub struct StatBot {
    settings: Mutex<Settings>
}

impl StatBot {
    pub fn new(settings: Settings) -> Self {
        Self{ settings: Mutex::new(settings) }
    }

    fn stats_subroutine(&self, ctx: &Context, msg: &Message, args: &[&str]) {
        if args.len() > 0 {
            msg.channel_id
                .send_message(&ctx, |m| m.content("Error: stats does not expect args"))
                .unwrap();
        } else{
            let mut st = STATS.lock().unwrap();
            st.update_stats();

            msg.channel_id
                .send_message(&ctx, |m| m.content(st.as_human_readable_string()))
                .unwrap();
        }
    }

    fn settings_subroutine(&self, settings: &mut Settings, ctx: &Context, msg: &Message, args: &[&str]) {
        if args.len() == 0 {
            msg.channel_id
                .send_message(&ctx, |m| m.content(format!("{}settings prefix", settings.prefix)))
                .unwrap();
        } else {
            if args[0] == "prefix" {
                if args.len() == 2 {
                    settings.prefix = args[1].to_string();

                    {
                        let outdir = OUTPUT_DIR.lock().unwrap();

                        let f = File::create(format!("{}/{}", outdir, SETTINGS_FILE_NAME)).unwrap();
                        settings.store(f).unwrap();
                    }

                    msg.channel_id
                        .send_message(&ctx, |m| m.content(format!("sucess: prefix is now '{}'", settings.prefix)))
                        .unwrap();

                } else {
                    msg.channel_id
                        .send_message(&ctx, |m| m.content("Error: settings prefix requires 1 arg"))
                        .unwrap();
                }
            } else {
                msg.channel_id
                    .send_message(&ctx, |m| m.content("Error: invalid setting"))
                    .unwrap();
            }
        }
    }
}

impl EventHandler for StatBot {
    fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            let mut settings = self.settings.lock().unwrap();

            if msg.content.starts_with(&settings.prefix) {
                let commandline = &msg.content[settings.prefix.len()..].split(" ")
                    .collect::<Vec<&str>>();

                if commandline.len() == 0 {
                    msg.channel_id
                        .send_message(&ctx, |m| m.content("Error: expected command"))
                        .unwrap();
                } else {
                    let cmd = commandline[0];
                    let args = &commandline[1..];

                    match cmd {
                        "stats" => self.stats_subroutine(&ctx, &msg, &args[..]),
                        "settings" => self.settings_subroutine(&mut settings, &ctx, &msg, &args[..]),
                        _ => (),
                    }
                }
            }
        }
    }

    fn ready(&self, ctx: Context, rdy: Ready) {
        let mut st = STATS.lock().unwrap();
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

        println!("<{}> scan complete, now online", Utc::now().format("%Y-%m-%d_%H:%M:%S"));
    }

    fn voice_state_update(&self, ctx: Context, _: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
        let mut st = STATS.lock().unwrap();

        let date_time = Utc::now().format("%Y-%m-%d_%H:%M:%S");

        match new.channel_id {
            Some(_) if !new.deaf && !new.self_deaf => {
                st.user_now_online(new.user_id);
                println!("<{}> User joined: {}", date_time, new.user_id.to_user(ctx).unwrap().name);
            },
            _ => {
                st.user_now_offline(new.user_id);
                println!("<{}> User left: {}", date_time, new.user_id.to_user(&ctx).unwrap().name);
            },
        }
    }
}