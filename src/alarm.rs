extern crate bitflags;
extern crate chrono;

use self::bitflags::bitflags;
use self::chrono::{DateTime,  Weekday, Duration, Local, Timelike, Datelike};

#[derive(Debug, Copy, Clone)]
pub struct Time {
    hour : u8,
    min : u8
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

enum AlarmMode {
    OneTime,
    Recurring(DayMask)
}

struct FadeMode {
    length_s  : Duration,
    start_vol : f32,
    end_vol   : f32
}

pub struct Alarm {
    enabled : bool,
    time    : Time,
    fading  : FadeMode,
    mode    : AlarmMode,
}

impl Alarm {
    pub fn new() -> Alarm {
        Alarm { enabled: true,
                time : Time { hour: 6, min: 45 },
                fading : FadeMode {length_s: Duration::seconds(10), start_vol: 0.1, end_vol: 0.7},
                 //Recurring(DayMask::default())},
                mode : AlarmMode::OneTime }
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
}
