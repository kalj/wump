
mod lcd;
mod buttons;
mod alarm;
extern crate chrono;
extern crate mpd;

use chrono::{Local, DateTime};

use lcd::Lcd;
use buttons::ButtonHandler;
use alarm::Alarm;
use alarm::AlarmMode;
use alarm::DayMask;
use alarm::Time;
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
// const MUTE_PIN: u64 = 16;
// const POFF_PIN: u64 = 26;

const BUTTONS: &[u64] = &[BUTTON_A, BUTTON_B, BUTTON_C, BUTTON_D];

const I2C_PATH: &str = "/dev/i2c-1";
const LCD_ADDR: u16 = 0x27;

#[derive(Copy, Clone)]
struct Fade {
    start_time: DateTime<Local>,
    end_time: DateTime<Local>,
    start_vol: f32,
    end_vol:f32
}

impl Fade {
    fn new(start: DateTime<Local>, alarm: &Alarm) -> Fade
    {
        Fade{start_time:start,
             end_time:start+alarm.get_length(),
             start_vol:alarm.get_start_vol(),
             end_vol:alarm.get_end_vol()}
    }
}

enum PlaybackState {
    Playing,
    Paused,
    Fading(Fade)
}

struct State {
    alarm : Alarm,
    pb_state : PlaybackState,
}

fn main()
{
    let mut state = State { alarm:    Alarm::new(true,
                                                 Time::new(6,45),
                                                 10, 0.1, 0.7, AlarmMode::Recurring(DayMask::default())),
                            pb_state: PlaybackState::Paused};

    let mut button_handler = ButtonHandler::new(BUTTONS);

    let mut lcd = Lcd::new(I2C_PATH, LCD_ADDR);

    // Initialise display
    lcd.init();

    // Send some test
    lcd.set_lines("Wake-Up MP 0.3","Starting up...");

    // 1 second delay
    thread::sleep(Duration::new(1,0));

    let backlight_timeout : chrono::Duration = chrono::Duration::seconds(5);
    let mut last_button_activity = Local::now();

    loop {
        let now : DateTime<Local> = Local::now();
        let mut mpd_conn = mpd::Client::connect("127.0.0.1:6600").expect("Failed connecting to mpd");
        let mpd_state = mpd_conn.status().expect("Failed querying mpd for status").state;

        // update state based on external mpd state changes
        state.pb_state = match mpd_state {
            mpd::State::Stop|mpd::State::Pause => PlaybackState::Paused,
            mpd::State::Play => match state.pb_state {
                PlaybackState::Paused => PlaybackState::Playing,
                PlaybackState::Playing|PlaybackState::Fading(_) => state.pb_state
            }
        };

        // gather button events
        let mut button_toggle_alarm_enabled = false;
        let mut button_toggle_play = false;
        let mut button_activity = false;

        button_handler.handle_events(|x| {

            if x == BUTTON_B {
                button_toggle_play = true;
                println!("Toggle play button pressed");
            }
            if x == BUTTON_A {
                button_toggle_alarm_enabled = true;
                println!("Toggle alarm state button pressed, alarm at {:?}",state.alarm.get_time());

            }

            button_activity = true;

            if x == BUTTON_D {
                println!("Toggle backlight state button pressed");
                if lcd.get_backlight() {
                    button_activity = false;
                    last_button_activity = now-(backlight_timeout+backlight_timeout);
                }
                // other case handled by activity = true above
            }
        });

        // handle button events and alarm state changes

        if button_toggle_alarm_enabled {
            state.alarm.toggle_enabled();
        }

        if state.alarm.should_start(&now) {
            match state.pb_state {
                PlaybackState::Paused => {
                    println!("Starting up the alarm!");
                    state.alarm.start();
                    state.pb_state=PlaybackState::Fading(Fade::new(now,&state.alarm));
                }
                _ => {}
            }
        }

        if button_toggle_play {
            state.pb_state = match state.pb_state {
                PlaybackState::Paused => PlaybackState::Playing,
                PlaybackState::Playing|PlaybackState::Fading(_) => PlaybackState::Paused
            };
        }

        // if button=change_volume => { set volume, and if state==fading => state = playing }

        // handle fading, set volume or change

        if let PlaybackState::Fading(fade) = state.pb_state {
            println!("start_time: {}, end_time: {}, now: {}", fade.start_time, fade.end_time, now);
            let num = (now-fade.start_time).num_milliseconds() as f32;
            let den = (fade.end_time - fade.start_time).num_milliseconds() as f32;
            let a = (num / den).min(1.0).max(0.0);

            let vol_fraction = fade.start_vol + (fade.end_vol-fade.start_vol)*a;
            let vol_percent = (vol_fraction*100.0).round() as i8;

            println!("Fading. a={}, setting volume to {}",a, vol_percent);
            mpd_conn.volume(vol_percent).expect("Failed sending set volume command to mpd.");
            if now >= fade.end_time {
                println!("Done fading, switching to PlaybackState::Playing");
                state.pb_state = PlaybackState::Playing;
            }
        }

        // handle playback state changes (due to button press or alarm starting)
        match mpd_state {
            mpd::State::Stop|mpd::State::Pause => match state.pb_state {
                PlaybackState::Playing|PlaybackState::Fading(_) => mpd_conn.play().expect("Failed sending play command to mpd."),
                _ => ()
            },
            mpd::State::Play => match state.pb_state {
                PlaybackState::Paused => mpd_conn.pause(true).expect("Failed sending pause command to mpd."),
                _ => ()
            }
        };

        // handle backlight state toggle
        if button_activity {
            last_button_activity = now;
            lcd.set_backlight(true);
        } else {
            let dur = now.signed_duration_since(last_button_activity);
            if dur > backlight_timeout {
                lcd.set_backlight(false);
            }
        }

        let pb_state_char = if let PlaybackState::Paused = state.pb_state { " " } else { ">" };
        let alarm_str = state.alarm.to_str();
        let l1 = format!("{:<11}{}",alarm_str,now.format("%H:%M"));
        let l2 = format!("{:<16}", pb_state_char);
        lcd.set_lines(&l1,&l2);

        thread::sleep(Duration::from_millis(250));
    }
}
