
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

// Pin usage of Hifiberry Miniamp:
// GPIOs 18-21 (pins 12, 35, 38 and 40) are used for the sound
// interface. GPIO16 can be used to mute the power stage. GPIO26 shuts
// down the power stage. You can’t use these GPIOs for any other
// purpose.

// Pin usage of LCD (i2c)
// GPIO2 (SDA) & GPIO3 (SCL) (pins 3 & 5)

const BUTTON_A: u64 = 22; // Red
const BUTTON_B: u64 = 23; // Yellow
const BUTTON_C: u64 = 24; // Blue
const BUTTON_D: u64 = 25; // Green

const BUTTONS: &[u64] = &[BUTTON_A, BUTTON_B, BUTTON_C, BUTTON_D];

const I2C_PATH: &str = "/dev/i2c-1";
const LCD_ADDR: u16 = 0x27;

enum PlaybackState {
    Play,
    Pause
}

struct State {
    pb_state : PlaybackState,
}



fn set_pin_dir(pin: & Pin, dir: Direction) -> std::result::Result<(), retry::Error<sysfs_gpio::Error>>
{
    retry(delay::Fixed::from_millis(100).take(3), || pin.set_direction(dir))
}

fn main()
{
    let mut state = State { pb_state : PlaybackState::Pause};

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
    lcd.set_lines("Wake-Up MP 0.3","Starting up...");

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

    let lifetime : chrono::Duration = chrono::Duration::seconds(5);
    let mut lastactivity = Local::now();

    loop {
        let now : DateTime<Local> = Local::now();
        let mut mpd_conn = mpd::Client::connect("127.0.0.1:6600").expect("Failed connecting to mpd");
        match mpd_conn.status().expect("Failed querying mpd for status").state {
            mpd::State::Stop => {
                state.pb_state = PlaybackState::Pause;
            }
            mpd::State::Play => {
                state.pb_state = PlaybackState::Play;
            }
            mpd::State::Pause => {
                state.pb_state = PlaybackState::Pause;
            }
        }

        let mut activity = false;
        let mut toggle_play = false;

        for x in rx.try_iter() {
            if x == BUTTON_B {
                toggle_play = true;
                println!("Toggle play button pressed");
            }

            activity = true;

            if x == BUTTON_D {
                println!("Toggle backlight state button pressed");
                if lcd.get_backlight() {
                    activity = false;
                    lastactivity = now-(lifetime+lifetime);
                }
                // other case handled by activity = true above
            }
        }

        if let PlaybackState::Pause = state.pb_state  {
            println!("Is not playing...");
        }


        if activity {
            lastactivity = now;
            lcd.set_backlight(true);
        } else {
            let dur = now.signed_duration_since(lastactivity);
            if dur > lifetime {
                lcd.set_backlight(false);
            }
        }

        let l2 = "a";
        lcd.set_lines(&now.format("     %H:%M      ").to_string(),&l2);

        if toggle_play {
            let mut do_steps = || -> mpd::error::Result<_> {
                match state.pb_state {
                    PlaybackState::Play => {
                        mpd_conn.pause(true)?;
                        state.pb_state = PlaybackState::Pause;
                    }
                    PlaybackState::Pause => {
                        mpd_conn.play()?;
                        state.pb_state = PlaybackState::Play;
                    }
                }
                Ok(())
            };

            if let Err(e) = do_steps() {
                println!("Failed toggling play state ({})",e);
            }
        }

        thread::sleep(Duration::from_millis(250));
    }
}
