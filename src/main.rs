
#[macro_use]
extern crate rouille;
extern crate chrono;
extern crate mpd;

use std::time::Duration;
use std::thread;
use std::sync::{RwLock,Arc};

use chrono::{Local, DateTime};

mod fontmap;
mod oled;
mod buttons;
mod alarm;
mod webui;

use oled::Oled;
use buttons::ButtonHandler;
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
const BUTTON_A: u8 = 22; // Red
const BUTTON_B: u8 = 27; // Green
const BUTTON_D: u8 = 17; // Yellow

// const MUTE_PIN: u8 = 16;
// const POFF_PIN: u8 = 26;

const BUTTONS: &[u8] = &[BUTTON_A, BUTTON_B, BUTTON_D];

const OLED_DC_PIN_ID: u8  = 24;
const OLED_RST_PIN_ID: u8 = 23;
const OLED_SPI_BUS: rppal::spi::Bus        = rppal::spi::Bus::Spi0;
const OLED_SPI_SS: rppal::spi::SlaveSelect = rppal::spi::SlaveSelect::Ss1;

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

    let mut button_handler = ButtonHandler::new(BUTTONS);

    let webui = start_webui(state.alarm.clone());

    let mut oled = Oled::new(OLED_DC_PIN_ID, OLED_RST_PIN_ID, OLED_SPI_BUS, OLED_SPI_SS);

    // Initialise display
    oled.init();

    // Send some test
    oled.set_top_line("Wake-Up MP 0.4");
    oled.set_bottom_line("Starting up...");

    // 1 second delay
    thread::sleep(Duration::new(1,0));

    let dim_timeout : chrono::Duration = chrono::Duration::seconds(5);
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
                println!("Toggle alarm state button pressed");

            }

            button_activity = true;

            if x == BUTTON_C {
                println!("Toggle dimmed state button pressed");
                if !oled.get_dimmed() {
                    button_activity = false;
                    last_button_activity = now-(dim_timeout+dim_timeout);
                }
                // other case handled by activity = true above
            }
        });

        // handle button events and alarm state changes

        if button_toggle_alarm_enabled {
            state.alarm.write().unwrap().toggle_enabled();
        }

        {
            let mut alarm = state.alarm.write().unwrap();
            if alarm.should_start(&now) {
                match state.pb_state {
                    PlaybackState::Paused => {
                        println!("Starting up the alarm!");
                        alarm.start();
                        state.pb_state=PlaybackState::Fading(Fade::new(now,&alarm));
                    }
                    _ => {}
                }
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

        // handle dimmed state toggle
        if button_activity {
            last_button_activity = now;
            oled.set_dimmed(false);
        } else {
            let dur = now.signed_duration_since(last_button_activity);
            if dur > dim_timeout {
                oled.set_dimmed(true);
            }
        }

        let l1 = if let PlaybackState::Paused = state.pb_state { "Paused" } else { "Playing" };
        let alarm_str = state.alarm.read().unwrap().to_str();
        let l2 = format!("Alarm: {}", alarm_str);

        oled.show_time(&now);
        oled.set_top_line(&l1);
        oled.set_bottom_line(&l2);

        thread::sleep(Duration::from_millis(250));
    }
}
