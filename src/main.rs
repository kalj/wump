
mod lcd;
extern crate chrono;

use chrono::{Local, DateTime};

use lcd::Lcd;
use std::time::Duration;
use std::thread::sleep;
use std::io::{self, Write};

const I2C_PATH: &str = "/dev/i2c-1";
const LCD_ADDR: u16 = 0x27;

fn main() {
    let mut lcd = Lcd::new(I2C_PATH, LCD_ADDR);

    // Initialise display
    lcd.init();

    // Send some test
    lcd.set_lines("Alarm Clock 0.3","Starting up...");

    // 1 second delay
    sleep(Duration::new(1,0));

    loop {
        let now : DateTime<Local> = Local::now();

        lcd.set_lines(&now.format("     %H:%M      ").to_string(),"");

        sleep(Duration::new(1,0));
    }
}
