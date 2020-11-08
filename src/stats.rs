use chrono::{Utc, Date};

use serenity::model::id::UserId;

use std::collections::BTreeMap;
use std::time::{Instant, Duration};
use std::sync::Arc;
use serenity::CacheAndHttp;
use std::num::ParseIntError;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::process::Command;
use tempfile::{Builder, TempPath};

static DATE_FMT_STR: &str = "%Y-%m-%d";


#[derive(Debug)]
pub enum StatParseError {
    UserIdParseError(ParseIntError),
    JsonParseError(serde_json::Error),
    IOError(std::io::Error),
}

impl From<ParseIntError> for StatParseError {
    fn from(e: ParseIntError) -> Self {
        StatParseError::UserIdParseError(e)
    }
}

impl From<serde_json::Error> for StatParseError {
    fn from(e: serde_json::Error) -> Self {
        StatParseError::JsonParseError(e)
    }
}

impl From<std::io::Error> for StatParseError {
    fn from(e: std::io::Error) -> Self {
        StatParseError::IOError(e)
    }
}


#[derive(Clone, Default)]
pub struct StatManager {
    output_dir: PathBuf,
    graphing_tool_path: PathBuf,
    online_time: BTreeMap<UserId, Duration>,
    online_since: BTreeMap<UserId, Instant>,
    cache_and_http: Arc<CacheAndHttp>,
}

impl StatManager {
    fn stat_file_path(&self, date: Date<Utc>) -> PathBuf {
        self.output_dir
            .join(format!("stats_{}.json", date.format(DATE_FMT_STR)))
    }

    fn trans_file_path(&self) -> PathBuf {
        self.output_dir
            .join("trans.json")
    }

    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            graphing_tool_path: Default::default(),
            online_time: Default::default(),
            online_since: Default::default(),
            cache_and_http: Default::default()
        }
    }

    pub fn set_output_dir<P: AsRef<Path>>(&mut self, outdir: P) {
        self.output_dir = outdir.as_ref().to_path_buf();
    }

    pub fn set_graphing_tool_path<P: AsRef<Path>>(&mut self, path: P) {
        self.graphing_tool_path = path.as_ref().to_path_buf();
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
            .filter_map(|(uid, _)| uid.clone().to_user(&self.cache_and_http).ok())
            .map(|user| (user.id, user.name))
            .collect()
    }

    fn get_stat_impl<R: Read>(rdr: R) -> Result<BTreeMap<UserId, Duration>, StatParseError> {
        let j: BTreeMap<String, u64> = serde_json::from_reader(rdr)?;

        j.into_iter()
            .map(|(uid, secs)| {
                let parsed_uid = uid.parse::<u64>()?;
                Ok((UserId::from(parsed_uid), Duration::from_secs(secs)))
            })
            .collect()
    }

    pub fn get_stats_unbuffered(&self, date: Date<Utc>) -> Result<BTreeMap<UserId, Duration>, StatParseError> {
        let f = File::open(self.stat_file_path(date))?;
        Self::get_stat_impl(f)
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
                let f = File::open(fp)?;
                let newest_st = Self::get_stat_impl(f)?;

                newest_st
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
                .map(|(uid, ontime)| (format!("{}", uid), ontime.as_secs()))
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
                .duration_since(timestamp.clone());

            match self.online_time.get_mut(&uid) {
                Some(t) => { *t += duration; },
                None    => { self.online_time.insert(uid.clone(), duration); }
            }

            *timestamp = Instant::now();
        }
    }

    pub fn user_now_offline(&mut self, uid: UserId) -> bool {
        match self.online_since.remove(&uid) {
            Some(since) => {
                let duration = Instant::now()
                    .duration_since(since);

                match self.online_time.get_mut(&uid) {
                    Some(time) => { *time += duration; },
                    None => { self.online_time.insert(uid, duration); },
                }

                true
            },
            None => false
        }
    }

    pub fn user_now_online(&mut self, uid: UserId) -> bool {
        if !self.online_since.contains_key(&uid) {
            match self.online_since.insert(uid, Instant::now()) {
                None => true,
                Some(_) => false,
            }
        } else {
            false
        }
    }

    pub fn generate_graph(&self, total: bool) -> std::io::Result<TempPath> {
        let tmp_file_path = Builder::new()
            .suffix(".png")
            .tempfile()?
            .into_temp_path();

        const ARGS: [&'static str; 11] = [
            "-x", "6",
            "-y", "10",
            "-n", "1080",
            "-m", "1920",
            "-s", "2020-05-11",
            "-t"
        ];

        let output = Command::new(&self.graphing_tool_path)
            .args(if total { &ARGS } else { &ARGS[..ARGS.len() - 1] })
            .arg(&self.output_dir)
            .arg(&tmp_file_path)
            .output()?;

        if !output.status.success() {
            match String::from_utf8(output.stdout) {
                Ok(emsg) => eprintln!("E: stat-graphing failed with output:\n{}", emsg),
                Err(e) => eprintln!("E: stat-graphing failed but could not decode output")
            }
        }

        Ok(tmp_file_path)
    }


}
