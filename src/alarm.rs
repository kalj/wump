extern crate bitflags;
extern crate chrono;

use self::bitflags::bitflags;
use self::chrono::{DateTime,  Weekday, Duration, Local, Timelike, Datelike};

#[derive(Debug, Copy, Clone)]
pub struct Time {
    hour : u8,
    min : u8
}

impl Time {
    pub fn new(hour: u8, min: u8) -> Time {
        Time { hour: hour, min: min }
    }
}


// time to str: "{:02}:{:02}",time.hour,time.min

bitflags! {
    pub struct DayMask: u8 {
        const MONDAY =    0b0000_0001;
        const TUESDAY =   0b0000_0010;
        const WEDNESDAY = 0b0000_0100;
        const THURSDAY =  0b0000_1000;
        const FRIDAY =    0b0001_0000;
        const SATURDAY =  0b0010_0000;
        const SUNDAY =    0b0100_0000;
    }
}
impl Default for DayMask {
    fn default() -> Self {
        Self::all() - DayMask::SATURDAY - DayMask::SUNDAY
    }
}
impl DayMask {
    pub fn contains_dow(self, dow: Weekday) -> bool {
        match dow {
            Weekday::Mon => self.contains(DayMask::MONDAY),
            Weekday::Tue => self.contains(DayMask::TUESDAY),
            Weekday::Wed => self.contains(DayMask::WEDNESDAY),
            Weekday::Thu => self.contains(DayMask::THURSDAY),
            Weekday::Fri => self.contains(DayMask::FRIDAY),
            Weekday::Sat => self.contains(DayMask::SATURDAY),
            Weekday::Sun => self.contains(DayMask::SUNDAY),
        }
    }
}

pub enum AlarmMode {
    OneTime,
    Recurring(DayMask)
}

pub struct Alarm {
    enabled   : bool,
    time      : Time,
    length    : Duration,
    start_vol : f32,
    end_vol   : f32,
    mode      : AlarmMode,
}

impl Alarm {
    pub fn new(enabled: bool, time: Time, length_s: i64, start_vol: f32, end_vol: f32, mode: AlarmMode) -> Alarm {
        Alarm { enabled: enabled,
                time: time,
                length: Duration::seconds(length_s),
                start_vol: start_vol,
                end_vol: end_vol,
                mode: mode }
    }

    pub fn to_str(&self) -> String {
        if self.enabled {
            match self.mode {
                AlarmMode::OneTime => {
                    format!("a:{:02}:{:02}:S",self.time.hour,self.time.min)
                },
                AlarmMode::Recurring(dm) => {
                    if dm==DayMask::from_bits_truncate(0b0001_1111) {
                        format!("a:{:02}:{:02}:M-F",self.time.hour,self.time.min)
                    }
                    else {
                        format!("a:{:02}:{:02}:R",self.time.hour,self.time.min)
                    }
                }
            }
        }
        else {
            "".to_string()
        }
    }

    pub fn get_length(&self) -> Duration {
        self.length
    }

    pub fn get_start_vol(&self) -> f32 {
        self.start_vol
    }

    pub fn get_end_vol(&self) -> f32 {
        self.end_vol
    }

    pub fn is_enabled(&self) -> bool {
         self.enabled
    }

    pub fn toggle_enabled(&mut self) {
         self.enabled = !self.enabled;
    }

    pub fn get_time(&self) -> Time {
         self.time
    }

    pub fn should_start(&self, datetime: &DateTime<Local>) -> bool {
        if !self.enabled ||
            datetime.second() != 0 ||
            datetime.hour() != self.time.hour.into() ||
            datetime.minute() != self.time.min.into() {
            return false;
        }

        match self.mode {
            AlarmMode::OneTime => true,
            AlarmMode::Recurring(mask) => mask.contains_dow(datetime.weekday())
        }
    }

    pub fn start(&mut self) {
        if let AlarmMode::OneTime = self.mode {
            self.enabled = false;
        }
    }
}
