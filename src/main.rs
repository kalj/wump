
mod lcd;
mod buttons;
mod alarm;
extern crate chrono;
extern crate mpd;

use chrono::{Local, DateTime};

use lcd::Lcd;
use buttons::ButtonHandler;
use alarm::Alarm;
use std::time::Duration;
use std::thread;

// Pin usage of Hifiberry Miniamp:
// GPIOs 18-21 (pins 12, 35, 38 and 40) are used for the sound
// interface. GPIO16 can be used to mute the power stage. GPIO26 shuts
// down the power stage. You can’t use these GPIOs for any other
// purpose.

// Pin usage of LCD (i2c)
// GPIO2 (SDA) & GPIO3 (SCL) (pins 3 & 5)

// Button pins
const BUTTON_A: u64 = 22; // Red
const BUTTON_B: u64 = 23; // Yellow
const BUTTON_C: u64 = 24; // Blue
const BUTTON_D: u64 = 25; // Green

const BUTTONS: &[u64] = &[BUTTON_A, BUTTON_B, BUTTON_C, BUTTON_D];

const I2C_PATH: &str = "/dev/i2c-1";
const LCD_ADDR: u16 = 0x27;

enum PlaybackState {
    Play,
    Pause,
    Fade(DateTime<Local>)
}

struct State {
    alarm : Alarm,
    pb_state : PlaybackState,
}

fn main()
{
    let mut state = State { alarm : Alarm::new(),
                            pb_state : PlaybackState::Pause};

    let mut button_handler = ButtonHandler::new(BUTTONS);

    let mut lcd = Lcd::new(I2C_PATH, LCD_ADDR);

    // Initialise display
    lcd.init();

    // Send some test
    lcd.set_lines("Wake-Up MP 0.3","Starting up...");

    // 1 second delay
    thread::sleep(Duration::new(1,0));

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

        let mut start_alarm = false;
        let mut activity = false;
        let mut toggle_play = false;

        button_handler.handle_events(|x| {

            if x == BUTTON_B {
                toggle_play = true;
                println!("Toggle play button pressed");
            }
            if x == BUTTON_A {
                state.alarm.toggle_enabled();
                println!("Toggle alarm state button pressed, alarm at {:?}",state.alarm.get_time());

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
        });

        if let PlaybackState::Pause = state.pb_state  {
            println!("Is not playing...");
            if state.alarm.should_start(&now) {
                start_alarm = true;
                println!("Starting alarm...");
            }
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

        let l2 = if state.alarm.is_enabled() { "a" } else { "" };
        lcd.set_lines(&now.format("     %H:%M      ").to_string(),&l2);

        if start_alarm {
            state.pb_state = PlaybackState::Play;
            mpd_conn.play().expect("Failed starting playback for alarm");
        }
        else if toggle_play {
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
                    PlaybackState::Fade(fade_time) => ()
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
