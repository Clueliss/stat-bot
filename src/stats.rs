use chrono::Utc;

use serenity::model::id::UserId;
use serenity::prelude::Context;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::time::{Instant, Duration};


fn seconds_to_human_readable(s_total: u64) -> String {
    let d = s_total/86400;
    let h = (s_total - d * 86400)/3600;
    let m = ((s_total - d * 86400) - h * 3600)/60;
    let s = ((s_total - d * 86400) - h * 3600) - (m * 60);

    format!("*{}* ***D***, *{}* ***H***, *{}* ***M***, *{}* ***S***", d, h, m, s)
}

fn into_json(map: BTreeMap<UserId, Duration>) -> BTreeMap<String, u64> {
    map.into_iter()
        .map(|(uid, dur)| (format!("{}", uid), dur.as_secs()))
        .collect()
}


#[derive(Clone, Default)]
pub struct Stats {
    online_time: BTreeMap<UserId, Duration>,
    online_since: BTreeMap<UserId, Instant>
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_stats<F: Read>(&mut self, mut f: F) -> Result<(), std::io::Error> {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let mut j: BTreeMap<String, BTreeMap<UserId, u64>> = serde_json::from_reader(&mut f).unwrap_or(Default::default());

        let st_today = j.remove(&date).unwrap_or(Default::default());
        self.online_time = st_today.into_iter().map(|(uid, t)| (uid, Duration::new(t, 0))).collect();

        Ok(())
    }

    pub fn flush_stats<F: Write + Read>(&mut self, mut f: F) -> Result<(), std::io::Error> {
        self.update_stats();

        let mut existent: BTreeMap<String, BTreeMap<UserId, Duration>> = serde_json::from_reader(&mut f)
            .unwrap_or(Default::default());

        let date = Utc::now().format("%Y-%m-%d").to_string();

        match existent.get_mut(&date) {
            Some(st) => { *st = self.online_time.clone(); },
            None => { existent.insert(date.clone(), self.online_time.clone()); }
        }

        let jmap: BTreeMap<String, BTreeMap<String, u64>> = existent.into_iter()
            .map(|(s, m)| (s, into_json(m)))
            .collect();

        serde_json::to_writer(&mut f, &jmap)?;

        Ok(())
    }

    pub fn update_stats(&mut self) {
        for (uid, timestamp) in self.online_since.iter_mut() {
            let duration = Instant::now()
                .duration_since(timestamp.clone());

            match self.online_time.get_mut(&uid) {
                Some(t) => { *t += duration; },
                None    => { self.online_time.insert(uid.clone(), duration); }
            }

            *timestamp = Instant::now();
        }
    }

    pub fn as_human_readable_string(&self, ctx: &Context) -> String {
        let mut buf = "Time wasted:\n".to_string();

        for (uid, time) in self.online_time.iter() {
            buf += &format!("  {}:\n  - {}\n",
                            uid.to_user(ctx).unwrap().name,
                            seconds_to_human_readable(time.as_secs()));
        }

        buf
    }

   pub fn user_now_offline(&mut self, uid: UserId) {
        if self.online_since.contains_key(&uid) {
            let since = self.online_since.remove(&uid).unwrap();

            let duration = Instant::now()
                .duration_since(since);

            match self.online_time.get_mut(&uid) {
                Some(time) => { *time += duration; },
                None       => { self.online_time.insert(uid, duration); },
            }
        }
    }

    pub fn user_now_online(&mut self, uid: UserId) {
        if !self.online_since.contains_key(&uid) {
            self.online_since.insert(uid, Instant::now());
        }
    }

}