use std::collections::btree_map::{BTreeMap, Entry};
use std::fs::File;
use std::io::Read;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{Date, Utc};
use serenity::model::id::UserId;
use thiserror::Error;

const DATE_FMT_STR: &str = "%Y-%m-%d";

fn unwrap_username(uid: &UserId, username: Option<String>) -> String {
    username.unwrap_or(format!("{:?}", uid))
}


#[derive(Debug, Error)]
pub enum StatParseError {
    #[error("failed to parse user id")]
    UserIdParseError(#[from] ParseIntError),
    #[error("failed to parse json")]
    JsonParseError(#[from] serde_json::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
}


#[derive(Clone)]
pub struct StatManager {
    output_dir: PathBuf,
    online_time: BTreeMap<UserId, (String, Duration)>,
    online_since: BTreeMap<UserId, Instant>
}

impl StatManager {
    pub fn database_path(&self) -> &Path {
        &self.output_dir
    }

    fn stat_file_path(&self, date: Date<Utc>) -> PathBuf {
        self.output_dir
            .join(format!("stats_{}.json", date.format(DATE_FMT_STR)))
    }

    fn trans_file_path(&self) -> PathBuf {
        self.output_dir
            .join("trans.json")
    }

    pub fn new<OutDir>(output_dir: OutDir) -> Self
    where
        OutDir: AsRef<Path>,
    {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            online_time: Default::default(),
            online_since: Default::default()
        }
    }

    pub fn user_iter(&self) -> impl Iterator<Item=&UserId> {
        self.online_time.iter().map(|(uid, _)| uid)
    }

    pub fn stats_iter(&self) -> impl Iterator<Item=(&UserId, &(String, Duration))> {
        self.online_time.iter()
    }

    pub fn generate_translations(&self) -> BTreeMap<UserId, String> {
        self.online_time.iter()
            .map(|(uid, (username, _))| (*uid, username.clone()))
            .collect()
    }

    fn get_stat_impl<StatR: Read, TransR: Read>(srdr: StatR, trdr: TransR) -> Result<BTreeMap<UserId, (String, Duration)>, StatParseError> {
        let stats: BTreeMap<String, u64> = serde_json::from_reader(srdr)?;
        let trans: BTreeMap<u64, String> = serde_json::from_reader(trdr)?;

        stats.into_iter()
            .map(|(uid, secs)| {
                let parsed_uid = uid.parse::<u64>()?;
                let username = trans.get(&parsed_uid).cloned().unwrap_or(format!("{:?}", UserId(parsed_uid)));
                Ok((UserId::from(parsed_uid), (username, Duration::from_secs(secs))))
            })
            .collect()
    }

    pub fn get_stats_unbuffered(&self, date: Date<Utc>) -> Result<BTreeMap<UserId, (String, Duration)>, StatParseError> {
        let stats = File::open(self.stat_file_path(date))?;
        let trans = File::open(self.trans_file_path())?;
        Self::get_stat_impl(stats, trans)
    }

    pub fn read_stats(&mut self) -> Result<(), StatParseError> {

        let newest = std::fs::read_dir(&self.output_dir)?
            .filter_map(|de| de.ok())
            .map(|de| de.path())
            .filter(|p| !p.is_dir())
            .filter(|p| {
                let filen = p.file_name()
                    .and_then(|osfilename| osfilename.to_str());

                match filen {
                    Some(filename) => filename.starts_with("stats_"),
                    None => false
                }
            })
            .max();

        self.online_time = match newest {
            Some(fp) => {
                let stats = File::open(fp)?;
                let trans = File::open(self.trans_file_path())?;

                Self::get_stat_impl(stats, trans)?
            },
            None => Default::default(),
        };

        Ok(())
    }

    pub fn flush_stats(&mut self) -> Result<(), StatParseError> {
        self.update_stats();

        {
            let f = File::create(self.stat_file_path(Utc::today()))?;

            let new: BTreeMap<String, u64> = self.online_time
                .clone()
                .into_iter()
                .map(|(uid, (_username, ontime))| (format!("{}", uid), ontime.as_secs()))
                .collect();

            serde_json::to_writer(f, &new)?;
        }

        {
            let f = File::create(self.trans_file_path())?;

            let trans: BTreeMap<String, String> = self.generate_translations()
                .into_iter()
                .map(|(uid, name)| (format!("{}", uid), name))
                .collect();

            serde_json::to_writer(f, &trans)?;
        }

        Ok(())
    }

    pub fn update_stats(&mut self) {
        for (uid, timestamp) in self.online_since.iter_mut() {
            let duration = Instant::now()
                .duration_since(*timestamp);

            match self.online_time.get_mut(&uid) {
                Some((_, t)) => { *t += duration; },
                None    => { self.online_time.insert(*uid, (unwrap_username(&uid, None), duration)); }
            }

            *timestamp = Instant::now();
        }
    }

    pub fn user_now_offline(&mut self, uid: UserId, username: Option<String>) -> bool {

        let new_username = unwrap_username(&uid, username);

        match self.online_since.remove(&uid) {
            Some(since) => {
                let duration = Instant::now()
                    .duration_since(since);

                match self.online_time.get_mut(&uid) {
                    Some((u, t)) => {
                        *t += duration;

                        if u != &new_username {
                            *u = new_username;
                        }
                    },
                    None => { self.online_time.insert(uid, (new_username, duration)); },
                }

                true
            },
            None => false
        }
    }

    pub fn user_now_online(&mut self, uid: UserId, username: Option<String>) -> bool {

        let new_username = unwrap_username(&uid, username);

        match self.online_time.get_mut(&uid) {
            Some((name, _)) => {
                if name != &new_username {
                    *name = new_username;
                }
            },
            None => { self.online_time.insert(uid, (new_username, Duration::from_secs(0))); }
        }

        match self.online_since.entry(uid) {
            Entry::Vacant(entry) => {
                entry.insert(Instant::now());
                true
            },
            Entry::Occupied(_) => false
        }
    }

    pub fn force_username_update(&mut self, trans: BTreeMap<UserId, String>) {
        for (uid, new_name) in trans {
            if let Some((old_name, _)) = self.online_time.get_mut(&uid) {
                *old_name = new_name;
            }
        }
    }
}
