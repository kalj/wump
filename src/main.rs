
#[macro_use]
extern crate rouille;
extern crate chrono;
extern crate mpd;

use std::time::Duration;
use std::thread;
use std::sync::{RwLock,Arc};
use std::cmp::Ordering;

use chrono::{Local, DateTime};

mod display;
mod input;
mod alarm;
mod webui;

use display::Display;
use input::{InputHandler,InputEvent};
use alarm::Alarm;
use alarm::AlarmMode;
use alarm::DayMask;
use alarm::Time;
use webui::start_webui;

// Pin usage of Hifiberry Miniamp:
// GPIOs 18-21 (pins 12, 35, 38 and 40) are used for the sound
// interface. GPIO16 can be used to mute the power stage. GPIO26 shuts
// down the power stage. You can’t use these GPIOs for any other
// purpose.

// Button pins
const BUTTON_A: u8   = 22; // Red
const BUTTON_B: u8   = 27; // Green
const BUTTON_C: u8   = 17; // Yellow
const BUTTON_ROT: u8 = 4;  // Rotary encoder

// const MUTE_PIN: u8 = 16;
// const POFF_PIN: u8 = 26;

const BUTTONS: &[u8] = &[BUTTON_A, BUTTON_B, BUTTON_C, BUTTON_ROT];

// Rotary encoder
const ROTENC_A: u8 = 15;
const ROTENC_B: u8 = 14;


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
    alarm: Arc<RwLock<Alarm>>,
    pb_state: PlaybackState,
}

fn main()
{
    let mut state = State { alarm:    Arc::new(RwLock::new(Alarm::new(true, Time::new(6,45),
                                                                      10, 0.1, 0.7,
                                                                      AlarmMode::Recurring(DayMask::default())))),
                            pb_state: PlaybackState::Paused};

    let mut input_handler = InputHandler::new(BUTTONS, (ROTENC_A, ROTENC_B));

    let _webui = start_webui(state.alarm.clone());

    // Create and initialize display
    let mut dpy = Display::new().unwrap();

    // Send some test
    dpy.set_top_line("Wake-Up MP 0.5").unwrap();
    dpy.set_bottom_line("Starting up...").unwrap();

    // 1 second delay
    thread::sleep(Duration::new(1,0));

    let dim_timeout : chrono::Duration = chrono::Duration::seconds(5);
    let mut last_input_activity = Local::now();

    loop {
        let now : DateTime<Local> = Local::now();
        let mut mpd_conn = mpd::Client::connect("127.0.0.1:6600").expect("Failed connecting to mpd");
        let mpd_status = mpd_conn.status().expect("Failed querying mpd for status");
        let mut volume = mpd_status.volume;

        // update state based on external mpd state changes
        state.pb_state = match mpd_status.state {
            mpd::State::Stop|mpd::State::Pause => PlaybackState::Paused,
            mpd::State::Play => match state.pb_state {
                PlaybackState::Paused => PlaybackState::Playing,
                PlaybackState::Playing|PlaybackState::Fading(_) => state.pb_state
            }
        };

        // gather input events
        let mut input_toggle_alarm_enabled = false;
        let mut input_toggle_play = false;
        let mut input_activity = false;
        let mut vol_change: i8 = 0;

        input_handler.handle_events(|x| {

            if let InputEvent::Button(BUTTON_A) = x {
                input_toggle_alarm_enabled = true;
                println!("Toggle alarm state button pressed");

            }
            if let InputEvent::Button(BUTTON_B) = x {
                input_toggle_play = true;
                println!("Toggle play button pressed");
            }

            if let InputEvent::Button(BUTTON_ROT) = x {
                println!("Rotary encoder button pressed");
            }

            if let InputEvent::RotaryEncoder(inc) = x {
                match inc.cmp(&0) {
                    Ordering::Greater => println!("Rotary encoder turned clockwise"),
                    Ordering::Less => println!("Rotary encoder turned counter-clockwise"),
                    Ordering::Equal => ()
                }
                vol_change += inc;
            }

            input_activity = true;

            if let InputEvent::Button(BUTTON_C) = x {
                println!("Toggle backlight button pressed");
                if dpy.get_backlight() {
                    input_activity = false;
                    last_input_activity = now-(dim_timeout+dim_timeout);
                }
                // other case handled by activity = true above
            }
        });

        // handle input events and alarm state changes

        if input_toggle_alarm_enabled {
            state.alarm.write().unwrap().toggle_enabled();
        }

        {
            let mut alarm = state.alarm.write().unwrap();
            if alarm.should_start(&now) {
                if let PlaybackState::Paused = state.pb_state {
                    println!("Starting up the alarm!");
                    alarm.start();
                    state.pb_state=PlaybackState::Fading(Fade::new(now,&alarm));
                }
            }
        }

        if input_toggle_play {
            state.pb_state = match state.pb_state {
                PlaybackState::Paused => PlaybackState::Playing,
                PlaybackState::Playing|PlaybackState::Fading(_) => PlaybackState::Paused
            };
        }

        // if input=change_volume => { set volume, and if state==fading => state = playing }
        // volume change

        if vol_change != 0 {
            if let PlaybackState::Fading(_) = state.pb_state {
                state.pb_state = PlaybackState::Playing;
            }
            volume = (5*vol_change +volume).min(100).max(0);

            mpd_conn.volume(volume).unwrap();

        }

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
        match mpd_status.state {
            mpd::State::Stop|mpd::State::Pause => match state.pb_state {
                PlaybackState::Playing|PlaybackState::Fading(_) => mpd_conn.play().expect("Failed sending play command to mpd."),
                _ => ()
            },
            mpd::State::Play => if let PlaybackState::Paused = state.pb_state {
                mpd_conn.pause(true).expect("Failed sending pause command to mpd.")
            }
        };

        // handle backlight toggle
        if input_activity {
            last_input_activity = now;
            dpy.set_backlight(true).unwrap();
        } else {
            let dur = now.signed_duration_since(last_input_activity);
            if dur > dim_timeout {
                dpy.set_backlight(false).unwrap();
            }
        }

        let pbstring = if let PlaybackState::Paused = state.pb_state { "Paused" } else { "Playing" };
        let l1 = format!("Vol: {}    {}", volume, pbstring);
        let alarm_str = state.alarm.read().unwrap().to_str();
        let l2 = format!("Alarm: {}", alarm_str);

        dpy.show_time(&now).unwrap();
        dpy.set_top_line(&l1).unwrap();
        dpy.set_bottom_line(&l2).unwrap();

        thread::sleep(Duration::from_millis(250));
    }
}
