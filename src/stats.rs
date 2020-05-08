use chrono::Utc;

use serenity::model::id::UserId;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::time::{Instant, Duration};
use std::sync::Arc;
use serenity::CacheAndHttp;


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

    pub fn stats_iter(&self) -> std::collections::btree_map::Iter<UserId, Duration> {
        self.online_time.iter()
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


    pub fn flush_stats<F>(&mut self, mut f: F) -> Result<(), std::io::Error>
    where
        F: Read + Write,
    {
        self.update_stats();

        let date = Utc::now().format("%Y-%m-%d").to_string();

        let mut existent: BTreeMap<String, BTreeMap<String, u64>> = serde_json::from_reader(&mut f)
            .unwrap_or(Default::default());

        let new: BTreeMap<String, u64> = self.online_time.clone()
            .into_iter()
            .map(|(uid, dur)| (format!("{}", uid), dur.as_secs()))
            .collect();

        existent.insert(date, new);

        serde_json::to_writer(&mut f, &existent)?;

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

    pub fn user_now_offline(&mut self, uid: UserId) -> bool {
        if self.online_since.contains_key(&uid) {
            let since = self.online_since.remove(&uid).unwrap();

            let duration = Instant::now()
                .duration_since(since);

            match self.online_time.get_mut(&uid) {
                Some(time) => { *time += duration; },
                None       => { self.online_time.insert(uid, duration); },
            }

            true
        } else {
            false
        }
    }

    pub fn user_now_online(&mut self, uid: UserId) -> bool {
        if !self.online_since.contains_key(&uid) {
            self.online_since.insert(uid, Instant::now());
            true
        } else {
            false
        }
    }
}
