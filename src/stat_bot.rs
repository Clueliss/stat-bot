use serenity::model::channel::{Message, GuildChannel, ChannelType};
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, ChannelId, UserId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::{EventHandler, Context};

use crate::stats::*;

use std::collections::HashMap;
use std::fs::File;
use std::sync::Mutex;
use chrono::{Utc, Date};
use std::time::Duration;
use std::path::{PathBuf, Path};

use serde::{Deserialize, Serialize};

use clap::Clap;
use tempfile::TempDir;


pub static DEFAULT_PREFIX: &str = ">>";
static SETTINGS_CHOICES: [&str; 1] = ["prefix"];
static SETTINGS_CHOICES_DESCR: [&str; 1] = [":exclamation: prefix"];


lazy_static! {
    pub static ref STATS: Mutex<StatManager> = Mutex::new(StatManager::default());
}


fn seconds_to_discord_formatted(s_total: u64) -> String {
    let d = s_total/86400;
    let h = (s_total - d * 86400)/3600;
    let m = ((s_total - d * 86400) - h * 3600)/60;
    let s = ((s_total - d * 86400) - h * 3600) - (m * 60);

    format!("*{}* ***D***, *{}* ***H***, *{}* ***M***, *{}* ***S***", d, h, m, s)
}


#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub prefix: String,
    pub output_dir: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self{ prefix: DEFAULT_PREFIX.to_string(), output_dir: PathBuf::new() }
    }
}


pub struct StatBot {
    settings: Mutex<Settings>,
    settings_path: PathBuf
}

impl StatBot {
    pub fn new<P: AsRef<Path>>(settings_path: P, settings: Settings) -> Self {
        Self {
            settings: Mutex::new(settings),
            settings_path: settings_path.as_ref().to_path_buf()
        }
    }

    fn stats_subroutine(&self, ctx: &Context, msg: &Message, args: &[&str]) {

        if args.len() > 0 {
            if args[0] == "graph" {
                let mut st = STATS.lock().unwrap();
                st.update_stats();

                let path: PathBuf = st.generate_graph().expect("stat graphing failed");

                msg.channel_id
                    .send_files(&ctx, std::iter::once(path.to_str().unwrap()), |m| m)
                    .unwrap();
            } else {
                msg.channel_id
                    .send_message(&ctx, |mb| mb.content(":x: Error: unknown subcommand"))
                    .unwrap();
            }
        } else {
            let mut st = STATS.lock().unwrap();
            st.update_stats();

            msg.channel_id
                .send_message(&ctx, |m| m.embed(|e| {

                    e.title("Time Wasted");

                    let sorted: Vec<(UserId, Duration)> = {
                        let mut buf: Vec<(UserId, Duration)> = st.stats_iter()
                            .map(|(uid, t)| (uid.clone(), t.clone()))
                            .collect();

                        buf.sort_by(|(_, t1), (_, t2)| t2.cmp(t1));
                        buf
                    };

                    for (uid, time) in sorted {
                        let user = uid.to_user(ctx).unwrap();

                        e.field(user.name, seconds_to_discord_formatted(time.as_secs()), false);
                    }

                    e
                })).unwrap();
        }
    }

    fn settings_subroutine(&self, settings: &mut Settings, ctx: &Context, msg: &Message, args: &[&str]) {

        let reply_sucess = |mes: &str| {
            msg.channel_id
                .send_message(&ctx, |mb| mb.content(format!(":white_check_mark: Success: {}", mes)))
                .unwrap();
        };

        let reply_err = |mes: &str| {
            msg.channel_id
                .send_message(&ctx, |mb| mb.content(format!(":x: Error: {}", mes)))
                .unwrap();
        };

        if args.len() == 0 {
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.embed(|e| {

                        e.title("StatBot Settings")
                            .description(format!("Use the command format `{}settings <option>`", settings.prefix));

                        for (choice, descr) in SETTINGS_CHOICES.iter().zip(SETTINGS_CHOICES_DESCR.iter()) {
                            e.field(
                                descr,
                                format!("`{}settings {}`", settings.prefix, choice), true);
                        }

                        e
                    })
                }).unwrap();
        } else {
            // prefix
            if args[0] == SETTINGS_CHOICES[0] {
                if args.len() == 2 {
                    settings.prefix = args[1].to_string();

                    {
                        let f = File::create(&self.settings_path).unwrap();
                        serde_json::to_writer(f, &settings).unwrap();
                    }

                    reply_sucess(&format!("prefix is now '{}'", settings.prefix));
                } else {
                    reply_err("required exactly 1 arg");
                }
            } else {
                reply_err("invalid setting");
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

    fn voice_state_update(&self, ctx: Context, _: Option<GuildId>, _old: Option<VoiceState>, new: VoiceState) {
        let mut st = STATS.lock().unwrap();

        let date_time = Utc::now().format("%Y-%m-%d_%H:%M:%S");

        match new.channel_id {
            Some(id) if !id.name(&ctx).unwrap().starts_with("AFK") && !new.deaf && !new.self_deaf => {
                let state_changed = st.user_now_online(new.user_id);

                if state_changed {
                    println!("<{}> User joined: {}", date_time, new.user_id.to_user(ctx).unwrap().name);
                }
            },
            _ => {
                let state_changed = st.user_now_offline(new.user_id);

                if state_changed {
                    println!("<{}> User left: {}", date_time, new.user_id.to_user(&ctx).unwrap().name);
                }
            },
        }
    }
}
