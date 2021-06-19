use diesel::QueryResult;
use serenity::model::channel::{ChannelType, GuildChannel, Message};
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::{Context, EventHandler};

use crate::stats::*;

use chrono::{Date, Duration, Utc};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};

use plotters::prelude::{BitMapBackend, IntoDrawingArea};
use serde::{Deserialize, Serialize};

pub const DEFAULT_PREFIX: &str = ">>";
const SETTINGS_CHOICES: [&str; 1] = ["prefix"];
const SETTINGS_CHOICES_DESCR: [&str; 1] = [":exclamation: prefix"];

enum UserState {
    Online,
    Offline,
}

fn seconds_to_discord_formatted(s_total: i64) -> String {
    let d = s_total / 86400;
    let h = (s_total - d * 86400) / 3600;
    let m = ((s_total - d * 86400) - h * 3600) / 60;
    let s = ((s_total - d * 86400) - h * 3600) - (m * 60);

    format!(
        "*{}* ***D***, *{}* ***H***, *{}* ***M***, *{}* ***S***",
        d, h, m, s
    )
}

fn log_user_state_change(uid: &UserId, username: Option<&String>, state: UserState) {
    let now = Utc::now().format("%Y-%m-%d_%H:%M:%S");

    match username {
        Some(name) => match state {
            UserState::Online => println!("<{now}> User joined: {name}", now = now, name = name),
            UserState::Offline => println!("<{now}> User left: {name}", now = now, name = name),
        },
        None => match state {
            UserState::Online => {
                println!("<{now}> User joined: {uid}", now = now, uid = uid);
                eprintln!("  ^- E: failed to receive username for: {:?}", uid);
            }
            UserState::Offline => {
                println!("<{now}> User left: {uid}", now = now, uid = uid);
                eprintln!("  ^- E: failed to receive username for: {:?}", uid);
            }
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
        Self {
            prefix: DEFAULT_PREFIX.to_string(),
            output_dir: PathBuf::from("./data"),
        }
    }
}

pub struct StatBot {
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    stat_man: Arc<RwLock<StatManager>>,
}

impl StatBot {
    pub fn new<P: AsRef<Path>>(
        settings_path: P,
        settings: Settings,
        stat_man: Arc<RwLock<StatManager>>,
    ) -> Self {
        Self {
            settings: Mutex::new(settings),
            settings_path: settings_path.as_ref().to_path_buf(),
            stat_man,
        }
    }

    fn data_for_time_total_graph(
        stats: RwLockReadGuard<'_, StatManager>,
    ) -> QueryResult<(
        Range<Date<Utc>>,
        BTreeMap<UserId, Vec<(Date<Utc>, Duration)>>,
    )> {
        let range = stats.date_range()?;
        let stats = stats.absolute_sum_time_per_day_iter()?;

        Ok((range, stats))
    }

    fn data_for_time_per_day_graph(
        stats: RwLockReadGuard<'_, StatManager>,
    ) -> QueryResult<(
        Range<Date<Utc>>,
        BTreeMap<UserId, Vec<(Date<Utc>, Duration)>>,
    )> {
        let range = stats.date_range()?;
        let stats = stats.time_per_day_iter()?;

        Ok((range, stats))
    }

    fn stats_subroutine(&self, ctx: &Context, msg: &Message, args: &[&str]) {
        if !args.is_empty() {
            enum E {
                ArgErr,
                DBMSError(diesel::result::Error),
            }

            let temppath = tempfile::Builder::new()
                .suffix(".png")
                .tempfile()
                .unwrap()
                .into_temp_path();

            msg.channel_id.broadcast_typing(&ctx).unwrap();

            let maybe_ok = {
                self.stat_man
                    .write()
                    .unwrap()
                    .flush_stats()
                    .expect("could not flush stats");

                let mut drawing_area =
                    BitMapBackend::new(&temppath, (1280, 720)).into_drawing_area();

                match &args {
                    &["graph", "total"] | &["graph"] => {
                        match Self::data_for_time_total_graph(self.stat_man.read().unwrap()) {
                            Ok((date_range, stats)) => {
                                crate::graphing::draw_graph(
                                    &mut drawing_area,
                                    ctx,
                                    "Time total",
                                    date_range,
                                    stats,
                                );

                                Ok(())
                            }
                            Err(e) => Err(E::DBMSError(e)),
                        }
                    }
                    &["graph", "time-per-day"] => {
                        match Self::data_for_time_per_day_graph(self.stat_man.read().unwrap()) {
                            Ok((date_range, stats)) => {
                                crate::graphing::draw_graph(
                                    &mut drawing_area,
                                    ctx,
                                    "Time per day",
                                    date_range,
                                    stats,
                                );

                                Ok(())
                            }
                            Err(e) => Err(E::DBMSError(e)),
                        }
                    }
                    _ => Err(E::ArgErr),
                }
            };

            match maybe_ok {
                Ok(_) => {
                    msg.channel_id
                        .send_files(&ctx, std::iter::once(temppath.to_str().unwrap()), |m| m)
                        .unwrap();
                }
                Err(E::ArgErr) => {
                    msg.channel_id
                        .send_message(&ctx, |mb| mb.content(":x: Error: unknown subcommand, ('time-per-day' is not implemented yet, if you tried that) "))
                        .unwrap();
                }
                Err(E::DBMSError(e)) => {
                    msg.channel_id
                        .send_message(&ctx, |mb| {
                            mb.content(":x: A database error occured while trying to draw graph")
                        })
                        .unwrap();

                    println!("E: stat graphing failed {:?}", e);
                }
            }
        } else {
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.embed(|e| {
                        self.stat_man
                            .write()
                            .unwrap()
                            .flush_stats()
                            .expect("could not flush stats");

                        e.title("Time Wasted");

                        let sorted = {
                            let mut buf: Vec<(UserId, Duration)> = self
                                .stat_man
                                .read()
                                .unwrap()
                                .absolute_sum_time_iter()
                                .unwrap()
                                .collect();

                            buf.sort_by(|(_, t1), (_, t2)| t1.cmp(t2));
                            buf
                        };

                        for (uid, dur) in sorted {
                            let secs = seconds_to_discord_formatted(dur.num_seconds());
                            e.field(uid.to_user(ctx).unwrap().name, secs, false);
                        }

                        e
                    })
                })
                .unwrap();
        }
    }

    fn settings_subroutine(
        &self,
        settings: &mut Settings,
        ctx: &Context,
        msg: &Message,
        args: &[&str],
    ) {
        let reply_sucess = |mes: &str| {
            msg.channel_id
                .send_message(&ctx, |mb| {
                    mb.content(format!(":white_check_mark: Success: {}", mes))
                })
                .unwrap();
        };

        let reply_err = |mes: &str| {
            msg.channel_id
                .send_message(&ctx, |mb| mb.content(format!(":x: Error: {}", mes)))
                .unwrap();
        };

        if args.is_empty() {
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.embed(|e| {
                        e.title("StatBot Settings").description(format!(
                            "Use the command format `{}settings <option>`",
                            settings.prefix
                        ));

                        for (choice, descr) in
                            SETTINGS_CHOICES.iter().zip(SETTINGS_CHOICES_DESCR.iter())
                        {
                            e.field(
                                descr,
                                format!("`{}settings {}`", settings.prefix, choice),
                                true,
                            );
                        }

                        e
                    })
                })
                .unwrap();
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
                let commandline = &msg.content[settings.prefix.len()..]
                    .split(' ')
                    .collect::<Vec<&str>>();

                if commandline.is_empty() {
                    msg.channel_id
                        .send_message(&ctx, |m| m.content("Error: expected command"))
                        .unwrap();
                } else {
                    let cmd = commandline[0];
                    let args = &commandline[1..];

                    match cmd {
                        "stats" => self.stats_subroutine(&ctx, &msg, args),
                        "settings" => self.settings_subroutine(&mut settings, &ctx, &msg, args),
                        "force-flush" => {
                            self.stat_man
                                .write()
                                .unwrap()
                                .flush_stats()
                                .expect("dbms error could not flush");
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn ready(&self, ctx: Context, rdy: Ready) {
        println!(
            "<{}> beginning scan",
            Utc::now().format("%Y-%m-%d_%H:%M:%S")
        );

        let tlof = rdy.guilds.get(0).unwrap();
        let channels: HashMap<ChannelId, GuildChannel> = tlof.id().channels(&ctx).unwrap();

        for (_id, ch) in channels {
            match ch.kind {
                ChannelType::Voice if !ch.name.starts_with("AFK") => match ch.members(&ctx) {
                    Ok(members) => {
                        for m in members {
                            match m.user_id().to_user(&ctx) {
                                Ok(user) => {
                                    if !user.bot {
                                        self.stat_man.write().unwrap().user_now_online(m.user_id());
                                    }
                                }
                                Err(e) => {
                                    eprintln!("E: could not determine if user with id {:?} is bot, counting anyways {:?}", m.user_id(), e);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        eprintln!("E: failed to enumerate members for channel {:?}", ch.name)
                    }
                },
                _ => (),
            }
        }

        println!(
            "<{}> scan complete, now online",
            Utc::now().format("%Y-%m-%d_%H:%M:%S")
        );
    }

    fn voice_state_update(
        &self,
        ctx: Context,
        _: Option<GuildId>,
        _old: Option<VoiceState>,
        new: VoiceState,
    ) {
        let username = new.user_id.to_user(&ctx).map(|u| u.name).ok();

        match new.channel_id {
            Some(id)
                if !id.name(&ctx).unwrap().starts_with("AFK") && !new.deaf && !new.self_deaf =>
            {
                let state_changed = self.stat_man.write().unwrap().user_now_online(new.user_id);

                if state_changed {
                    log_user_state_change(&new.user_id, username.as_ref(), UserState::Online);
                }
            }
            _ => {
                let state_changed = self
                    .stat_man
                    .write()
                    .unwrap()
                    .user_now_offline(new.user_id)
                    .unwrap();

                if state_changed {
                    log_user_state_change(&new.user_id, username.as_ref(), UserState::Offline);
                }
            }
        }
    }
}
