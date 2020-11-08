use serenity::model::channel::{Message, GuildChannel, ChannelType};
use serenity::model::gateway::Ready;
use serenity::model::id::{GuildId, ChannelId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::{EventHandler, Context};

use crate::stats::*;

use std::collections::HashMap;
use std::fs::File;
use std::sync::{Mutex, Arc};
use chrono::Utc;
use std::time::Duration;
use std::path::{PathBuf, Path};

use serde::{Deserialize, Serialize};

pub static DEFAULT_PREFIX: &str = ">>";
static SETTINGS_CHOICES: [&str; 1] = ["prefix"];
static SETTINGS_CHOICES_DESCR: [&str; 1] = [":exclamation: prefix"];

enum UserState {
    Online,
    Offline
}

fn seconds_to_discord_formatted(s_total: u64) -> String {
    let d = s_total/86400;
    let h = (s_total - d * 86400)/3600;
    let m = ((s_total - d * 86400) - h * 3600)/60;
    let s = ((s_total - d * 86400) - h * 3600) - (m * 60);

    format!("*{}* ***D***, *{}* ***H***, *{}* ***M***, *{}* ***S***", d, h, m, s)
}

fn log_user_state_change(uid: &UserId, username: Option<&String>, state: UserState) {

    let now = Utc::now().format("%Y-%m-%d_%H:%M:%S");

    match username {
        Some(name) => match state {
            UserState::Online  => println!("<{now}> User joined: {name}", now=now, name=name),
            UserState::Offline => println!("<{now}> User left: {name}", now=now, name=name),
        },
        None => match state {
            UserState::Online => {
                println!("<{now}> User joined: {uid}", now=now, uid=uid);
                eprintln!("  ^- E: failed to receive username for: {:?}", uid);
            },
            UserState::Offline => {
                println!("<{now}> User left: {uid}", now=now, uid=uid);
                eprintln!("  ^- E: failed to receive username for: {:?}", uid);
            },
        },
    }
}


#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub prefix: String,
    pub output_dir: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self{ prefix: DEFAULT_PREFIX.to_string(), output_dir: PathBuf::from("./data") }
    }
}


pub struct StatBot {
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    stat_man: Arc<Mutex<StatManager>>,
}

impl StatBot {
    pub fn new<P: AsRef<Path>>(settings_path: P, settings: Settings, stat_man: Arc<Mutex<StatManager>>) -> Self {
        Self {
            settings: Mutex::new(settings),
            settings_path: settings_path.as_ref().to_path_buf(),
            stat_man,
        }
    }

    fn stats_subroutine(&self, ctx: &Context, msg: &Message, args: &[&str]) {
        if args.len() > 0 {

            enum E {
                IOErr(std::io::Error),
                ArgErr,
            }

            let maybe_path = {
                let mut st = self.stat_man.lock().unwrap();
                st.update_stats();

                match &args {
                    &["graph", "total"] | &["graph"] => st.generate_graph(true).map_err(E::IOErr),
                    &["graph", "time-per-day"] => st.generate_graph(false).map_err(E::IOErr),
                    _ => Err(E::ArgErr)
                }
            };

            match maybe_path {
                Ok(path) => {
                    msg.channel_id
                        .send_files(&ctx, std::iter::once(path.to_str().unwrap()), |m| m)
                        .unwrap();
                },
                Err(E::ArgErr) => {
                    msg.channel_id
                        .send_message(&ctx, |mb| mb.content(":x: Error: unknown subcommand"))
                        .unwrap();
                },
                Err(E::IOErr(e)) => {
                    println!("E: stat graphing failed {:?}", e);
                }
            }


        } else {

            msg.channel_id
                .send_message(&ctx, |m| m.embed(|e| {

                    let mut st = self.stat_man.lock().unwrap();
                    st.update_stats();

                    e.title("Time Wasted");

                    let sorted = {
                        let mut buf: Vec<(UserId, (String, Duration))> = st.stats_iter()
                            .map(|(uid, t)| (uid.clone(), t.clone()))
                            .collect();

                        buf.sort_by(|(_, (_, t1)), (_, (_, t2))| t2.cmp(t1));
                        buf
                    };

                    for (_, (username, dur)) in sorted {
                        let secs = seconds_to_discord_formatted(dur.as_secs());
                        e.field(username, secs, false);
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
        let tlof = rdy.guilds.get(0).unwrap();
        let channels: HashMap<ChannelId, GuildChannel> = tlof.id().channels(&ctx).unwrap();

        let mut st = self.stat_man.lock().unwrap();

        for (_id, ch) in channels {
            match ch.kind {
                ChannelType::Voice if !ch.name.starts_with("AFK") => {

                    match ch.members(&ctx) {
                        Ok(members) => {
                            for m in members {

                                match m.user_id().to_user(&ctx) {
                                    Ok(user) => if !user.bot { st.user_now_online(m.user_id(), Some(user.name)); },
                                    Err(e) => { eprintln!("E: could not determine if user with id {:?} is bot, counting anyways {:?}", m.user_id(), e); }
                                }
                            }
                        },
                        Err(_) => {
                            eprintln!("E: failed to enumerate members for channel {:?}", ch.name)
                        }
                    }
                },
                _ => (),
            }
        }

        println!("<{}> scan complete, now online", Utc::now().format("%Y-%m-%d_%H:%M:%S"));
    }

    fn voice_state_update(&self, ctx: Context, _: Option<GuildId>, _old: Option<VoiceState>, new: VoiceState) {

        let username = new.user_id.to_user(&ctx).map(|u| u.name).ok();

        match new.channel_id {
            Some(id) if !id.name(&ctx).unwrap().starts_with("AFK") && !new.deaf && !new.self_deaf => {
                let state_changed = self.stat_man.lock().unwrap()
                    .user_now_online(new.user_id, username.clone());

                if state_changed {
                    log_user_state_change(&new.user_id, username.as_ref(), UserState::Online);
                }
            },
            _ => {
                let state_changed = self.stat_man.lock().unwrap()
                    .user_now_offline(new.user_id, username.clone());

                if state_changed {
                    log_user_state_change(&new.user_id, username.as_ref(), UserState::Offline);
                }
            },
        }
    }
}
