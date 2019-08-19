
mod lcd;
extern crate chrono;
extern crate sysfs_gpio;
extern crate retry;

use chrono::{Local, DateTime};

use lcd::Lcd;
use std::time::Duration;
use std::thread::sleep;
use sysfs_gpio::{Direction, Pin};
use retry::{retry,delay};

const BUTTON_A: u64 = 24;
const BUTTON_B: u64 = 23;

const I2C_PATH: &str = "/dev/i2c-1";
const LCD_ADDR: u16 = 0x27;

fn set_pin_dir(pin: & Pin, dir: Direction) -> std::result::Result<(), retry::Error<sysfs_gpio::Error>>
{
    retry(delay::Fixed::from_millis(100).take(3), || pin.set_direction(dir))
}

fn main()
{
    let button_a = Pin::new(BUTTON_A);
    button_a.export().expect("Failed exporting button A pin");
    set_pin_dir(&button_a, Direction::In).expect("Failed setting direction of A button");

    let button_b = Pin::new(BUTTON_B);
    button_b.export().expect("Failed exporting button B pin");
    set_pin_dir(&button_b, Direction::In).expect("Failed setting direction of B button");

    let mut lcd = Lcd::new(I2C_PATH, LCD_ADDR);

    // Initialise display
    lcd.init();

    // Send some test
    lcd.set_lines("Alarm Clock 0.3","Starting up...");

    // 1 second delay
    sleep(Duration::new(1,0));

    let lifetime : chrono::Duration = chrono::Duration::seconds(2);
    let mut lastactivity = Local::now();

    loop {
        let now : DateTime<Local> = Local::now();
        let dur = now.signed_duration_since(lastactivity);

        let a_val = button_a.get_value().expect("failed getting value of button A");
        let b_val = button_b.get_value().expect("failed getting value of button B");

        if a_val != 0 || b_val != 0 {
            lastactivity = now;
            lcd.set_backlight(true);
        } else if dur > lifetime {
            lcd.set_backlight(false);
        }

        lcd.set_lines(&now.format("     %H:%M      ").to_string(),"");

        sleep(Duration::from_millis(100));
    }
}
