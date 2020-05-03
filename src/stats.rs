use chrono::Utc;

use serenity::model::id::UserId;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::time::{Instant, Duration};
use std::sync::Arc;
use serenity::CacheAndHttp;


fn seconds_to_human_readable(s_total: u64) -> String {
    let d = s_total/86400;
    let h = (s_total - d * 86400)/3600;
    let m = ((s_total - d * 86400) - h * 3600)/60;
    let s = ((s_total - d * 86400) - h * 3600) - (m * 60);

    format!("*{}* ***D***, *{}* ***H***, *{}* ***M***, *{}* ***S***", d, h, m, s)
}


#[derive(Clone, Default)]
pub struct Stats {
    online_time: BTreeMap<UserId, Duration>,
    online_since: BTreeMap<UserId, Instant>,
    cache_and_http: Arc<CacheAndHttp>,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_cache_and_http(&mut self, ch: Arc<CacheAndHttp>) {
        self.cache_and_http = ch;
    }

    pub fn users(&self) -> Vec<UserId> {
        self.online_time.iter().map(|(uid, _)| uid.clone()).collect()
    }

    pub fn generate_translations(&self) -> BTreeMap<UserId, String> {
        self.online_time.iter()
            .map(|(uid, _)| (uid.clone(), uid.to_user(&self.cache_and_http).unwrap().name))
            .collect()
    }

    pub fn read_stats<F: Read>(&mut self, mut f: F) -> Result<(), std::io::Error> {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let mut j: BTreeMap<String, BTreeMap<UserId, u64>> = serde_json::from_reader(&mut f).unwrap_or_default();

        let st_today = j.remove(&date).unwrap_or_default();
        self.online_time = st_today.into_iter().map(|(uid, t)| (uid, Duration::new(t, 0))).collect();

        Ok(())
    }

    pub fn flush_stats<F: Write + Read>(&mut self, f: F) -> Result<(), std::io::Error> {
        self.flush_stats_map(f, |(uid, time)| (format!("{}", uid), time.as_secs()))
    }

    pub fn flush_stats_map<F, T, X>(&mut self, mut f: F, transform: T) -> Result<(), std::io::Error>
    where
        F: Read + Write,
        T: Fn((UserId, Duration)) -> (String, X),
        X: serde::ser::Serialize,
    {
        self.update_stats();

        let mut existent: BTreeMap<String, BTreeMap<UserId, Duration>> = serde_json::from_reader(&mut f)
            .unwrap_or(Default::default());

        let date = Utc::now().format("%Y-%m-%d").to_string();

        existent.insert(date, self.online_time.clone());

        let jmap: BTreeMap<String, BTreeMap<String, X>> = existent.into_iter()
            .map(move |(s, m)| (s, m.into_iter().map(&transform).collect()))
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

    pub fn as_human_readable_string(&self) -> String {
        let mut buf = "Time wasted:\n".to_string();

        let sorted_stats = {
            let mut tmp: Vec<(UserId, Duration)> = self.online_time.iter()
                .map(|(uid, time)| (uid.clone(), time.clone()))
                .collect();

            tmp.sort_by(|(_, t1), (_, t2)| t2.cmp(t1));
            tmp
        };

        for (uid, time) in sorted_stats {
            buf += &format!("  {}:\n  - {}\n",
                            uid.to_user(&self.cache_and_http).unwrap().name,
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
