extern crate bitflags;
extern crate chrono;

use self::bitflags::bitflags;
use self::chrono::{DateTime,  Weekday, Duration, Local, Timelike, Datelike};

#[derive(Debug, Copy, Clone)]
pub struct Time {
    hour: u8,
    min:  u8,
}

impl Time {
    pub fn new(hour: u8, min: u8) -> Time {
        Time { hour, min }
    }
    pub fn from_str(s: &str) -> Time {
        let mut time_iter = s.split(':');
        let hour = time_iter.next().unwrap().parse::<u8>().unwrap();
        let min  = time_iter.next().unwrap().parse::<u8>().unwrap();
        Time { hour, min }
    }
    pub fn to_str(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.min)
    }
}

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

#[derive(Copy, Clone)]
pub enum AlarmMode {
    OneTime,
    Recurring(DayMask),
}

#[derive(Copy, Clone)]
pub struct Alarm {
    enabled:   bool,
    time:      Time,
    length:    Duration,
    start_vol: f32,
    end_vol:   f32,
    mode:      AlarmMode,
}

impl Alarm {
    pub fn new(enabled: bool, time: Time, length_s: i64, start_vol: f32, end_vol: f32, mode: AlarmMode) -> Alarm {
        Alarm { enabled, time, length: Duration::seconds(length_s), start_vol, end_vol, mode }
    }

    pub fn to_str(&self) -> String {
        if self.enabled {
            match self.mode {
                AlarmMode::OneTime => {
                    format!("{} (1 time)", self.time.to_str())
                }
                AlarmMode::Recurring(dm) => {
                    if dm == DayMask::from_bits_truncate(0b0001_1111) {
                        format!("{} (M-F)", self.time.to_str())
                    } else {
                        let mut mask = String::from("[");
                        for i in 0..7 {
                            let next_char = if dm.contains(DayMask::from_bits_truncate(1 << i)) { 'x' } else { ' ' };
                            mask.push(next_char);
                        }
                        mask.push(']');
                        format!("{} ({})", self.time.to_str(), mask)
                    }
                }
            }
        } else {
            "Disabled".to_string()
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

    pub fn get_mode(&self) -> AlarmMode {
        self.mode
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
        if !self.enabled
            || datetime.second() != 0
            || datetime.hour() != self.time.hour as u32
            || datetime.minute() != self.time.min as u32
        {
            return false;
        }

        match self.mode {
            AlarmMode::OneTime => true,
            AlarmMode::Recurring(mask) => mask.contains_dow(datetime.weekday()),
        }
    }

    pub fn start(&mut self) {
        if let AlarmMode::OneTime = self.mode {
            self.enabled = false;
        }
    }
}
