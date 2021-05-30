extern crate serde_json;
extern crate serde;

use self::serde::{Deserialize, Serialize};

use std::fs::File;
use std::io;

use alarm::Alarm;

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub alarm: Alarm
}

impl Config {
    pub fn read(&mut self, fname: &str) -> io::Result<()> {
        let file = File::open(fname)?;
        let reader = io::BufReader::new(file);

        *self = serde_json::from_reader(reader)?;
        Ok(())
    }

    pub fn write(&self, fname: &str) -> io::Result<()> {
        let file = File::create(fname)?;
        let writer = io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer,self).map_err(|e| e.into())
    }

    pub fn read_new(fname: &str) -> io::Result<Config> {
        let mut conf = Config::default();

        conf.read(fname)?;
        Ok(conf)
    }
}
