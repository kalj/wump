
mod lcd;
extern crate chrono;
extern crate sysfs_gpio;
extern crate retry;
extern crate mpd;

use chrono::{Local, DateTime};

use lcd::Lcd;
use std::time::Duration;
use std::thread;
use std::sync::mpsc;
use sysfs_gpio::{Direction, Pin, Edge};
use retry::{retry,delay};

const BUTTON_A: u64 = 24;
const BUTTON_B: u64 = 23;

const BUTTONS: &[u64] = &[BUTTON_A, BUTTON_B];

const I2C_PATH: &str = "/dev/i2c-1";
const LCD_ADDR: u16 = 0x27;

fn set_pin_dir(pin: & Pin, dir: Direction) -> std::result::Result<(), retry::Error<sysfs_gpio::Error>>
{
    retry(delay::Fixed::from_millis(100).take(3), || pin.set_direction(dir))
}

fn main()
{
    let (tx, rx) = mpsc::channel();

    let buttons : Vec<(u64,Pin)> = BUTTONS.iter().map(|&b| {
        let button = Pin::new(b);
        button.export().expect("Failed exporting pin");
        set_pin_dir(&button, Direction::In).expect("Failed setting direction of pin");
        button.set_edge(Edge::RisingEdge).expect("Failed setting edge of pin");
        (b,button)
    }).collect();


    let mut lcd = Lcd::new(I2C_PATH, LCD_ADDR);

    // Initialise display
    lcd.init();

    // Send some test
    lcd.set_lines("Alarm Clock 0.3","Starting up...");

    // 1 second delay
    thread::sleep(Duration::new(1,0));

    // start up threads
    let thread_handles : Vec<thread::JoinHandle<_>> = buttons.iter().map(|&(b,but)| {
        let tx_b = tx.clone();
        thread::spawn(move || {
            let mut poller = but.get_poller().expect("Failed getting poller.");
            loop {
                match poller.poll(std::isize::MAX).expect("Error occured in poll") {
                    Some(_) => {
                        // Do poor-mans debouncing
                        thread::sleep(Duration::from_millis(20));
                        let val = but.get_value().expect("Failed reading pin value");
                        if val == 1 {
                            tx_b.send(b).expect("Failed sending value for pin");
                        }
                    },
                    None => ()
                }
            }
        })
    }).collect();

    let lifetime : chrono::Duration = chrono::Duration::seconds(2);
    let mut lastactivity = Local::now();

    loop {
        let now : DateTime<Local> = Local::now();
        let dur = now.signed_duration_since(lastactivity);

        let mut activity = false;
        let mut toggle_play = false;
        for x in rx.try_iter() {
            println!("Received {}",x);
            if x == BUTTON_B {
                toggle_play = true;
            }

            activity = true;
        }

        if activity {
            lastactivity = now;
            lcd.set_backlight(true);
        } else if dur > lifetime {
            lcd.set_backlight(false);
        }

        let l2 = "a";
        lcd.set_lines(&now.format("     %H:%M      ").to_string(),&l2);

        if toggle_play {

            let do_steps = || {
                let mut conn = mpd::Client::connect("127.0.0.1:6600")?;
                conn.toggle_pause()
            };

            if let Err(e) = do_steps() {
                println!("Failed toggling play state ({})",e);
            }
        }

        thread::sleep(Duration::from_millis(250));
    }
}
