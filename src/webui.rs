extern crate tinytemplate;
extern crate serde_json;
extern crate serde;
extern crate chrono;

use self::serde::Serialize;
use self::serde_json::Number;
use self::tinytemplate::TinyTemplate;

use std::io;
use std::thread;
use std::sync::{Arc,RwLock};

use alarm::Alarm;
use alarm::AlarmMode;
use alarm::DayMask;
use alarm::Time;

#[derive(Serialize)]
struct Context {
    alarm_enabled_checked: String,
    alarm_mode_onetime_checked: String,
    alarm_mode_recurring_checked: String,
    alarm_daymask_disabled: String,
    alarm_daymask_mon_checked: String,
    alarm_daymask_tue_checked: String,
    alarm_daymask_wed_checked: String,
    alarm_daymask_thu_checked: String,
    alarm_daymask_fri_checked: String,
    alarm_daymask_sat_checked: String,
    alarm_daymask_sun_checked: String,
    alarm_time: String,
    alarm_start_vol: Number,
    alarm_end_vol: Number,
    alarm_fade_length_s: Number
}


fn create_page(alarm: &Alarm) -> String {
    let mut tt = TinyTemplate::new();
    tt.add_template("form", TEMPLATE).expect("Failed adding template");

    let mut alarm_mode_recurring_checked = "".to_string();
    let mut alarm_mode_onetime_checked   = "".to_string();
    let mut alarm_daymask_disabled       = "".to_string();
    let mut alarm_daymask_mon_checked    = "".to_string();
    let mut alarm_daymask_tue_checked    = "".to_string();
    let mut alarm_daymask_wed_checked    = "".to_string();
    let mut alarm_daymask_thu_checked    = "".to_string();
    let mut alarm_daymask_fri_checked    = "".to_string();
    let mut alarm_daymask_sat_checked    = "".to_string();
    let mut alarm_daymask_sun_checked    = "".to_string();

    match alarm.get_mode() {
        AlarmMode::OneTime => {
            alarm_mode_onetime_checked   = "checked".to_string();
            alarm_daymask_disabled       = "disabled".to_string();
        }
        AlarmMode::Recurring(dm) => {
            alarm_mode_recurring_checked = "checked".to_string();

            if dm.contains(DayMask::MONDAY)    { alarm_daymask_mon_checked = "checked".to_string() }
            if dm.contains(DayMask::TUESDAY)   { alarm_daymask_tue_checked = "checked".to_string() }
            if dm.contains(DayMask::WEDNESDAY) { alarm_daymask_wed_checked = "checked".to_string() }
            if dm.contains(DayMask::THURSDAY)  { alarm_daymask_thu_checked = "checked".to_string() }
            if dm.contains(DayMask::FRIDAY)    { alarm_daymask_fri_checked = "checked".to_string() }
            if dm.contains(DayMask::SATURDAY)  { alarm_daymask_sat_checked = "checked".to_string() }
            if dm.contains(DayMask::SUNDAY)    { alarm_daymask_sun_checked = "checked".to_string() }
        }
    }

    let context = Context {
        alarm_enabled_checked: (if alarm.is_enabled() { "checked" } else { "" }).to_string(),
        alarm_mode_onetime_checked: alarm_mode_onetime_checked,
        alarm_mode_recurring_checked: alarm_mode_recurring_checked,
        alarm_daymask_disabled: alarm_daymask_disabled,
        alarm_daymask_mon_checked: alarm_daymask_mon_checked,
        alarm_daymask_tue_checked: alarm_daymask_tue_checked,
        alarm_daymask_wed_checked: alarm_daymask_wed_checked,
        alarm_daymask_thu_checked: alarm_daymask_thu_checked,
        alarm_daymask_fri_checked: alarm_daymask_fri_checked,
        alarm_daymask_sat_checked: alarm_daymask_sat_checked,
        alarm_daymask_sun_checked: alarm_daymask_sun_checked,
        alarm_time:                alarm.get_time().to_str(),
        alarm_fade_length_s:       Number::from_f64(alarm.get_length().num_seconds() as f64).unwrap(),
        alarm_start_vol:           Number::from_f64((alarm.get_start_vol()*100.0).round() as f64).unwrap(),
        alarm_end_vol:             Number::from_f64((alarm.get_end_vol()*100.0).round() as f64).unwrap(),
    };

    return tt.render("form", &context).expect("Failed rendering template");
}

pub fn start_webui(alarm: Arc<RwLock<Alarm>>) -> thread::JoinHandle<()> {
    thread::spawn( || {
        println!("Starting web UI server listening on 0.0.0.0:8000");

        rouille::start_server("0.0.0.0:8000", move |request| {
            rouille::log(&request, io::stdout(), || {


                router!(request,
                        (GET) (/) => {
                            let page = create_page(&alarm.read().unwrap());
                            rouille::Response::html(page)
                        },

                        (POST) (/) => {
                            // This is the route that is called when the user submits the form of the
                            // home page.

                            // We query the data with the `post_input!` macro. Each field of the macro
                            // corresponds to an element of the form.
                            // If the macro returns an error (for example if a field is missing, which
                            // can happen if you screw up the form or if the user made a manual request)
                            // we return a 400 response.
                            let data = try_or_400!(post_input!(request, {
                                alarm_enabled: bool,

                                alarm_time: String,
                                alarm_mode: String,
                                alarm_daymask_mon: bool,
                                alarm_daymask_tue: bool,
                                alarm_daymask_wed: bool,
                                alarm_daymask_thu: bool,
                                alarm_daymask_fri: bool,
                                alarm_daymask_sat: bool,
                                alarm_daymask_sun: bool,

                                alarm_start_vol: u8,
                                alarm_end_vol: u8,
                                alarm_fade_length_s: i64,
                            }));

                            let mode = if data.alarm_mode == "recurring" {
                                let mut mask = DayMask::empty();
                                if data.alarm_daymask_mon { mask |= DayMask::MONDAY; }
                                if data.alarm_daymask_tue { mask |= DayMask::TUESDAY; }
                                if data.alarm_daymask_wed { mask |= DayMask::WEDNESDAY; }
                                if data.alarm_daymask_thu { mask |= DayMask::THURSDAY; }
                                if data.alarm_daymask_fri { mask |= DayMask::FRIDAY; }
                                if data.alarm_daymask_sat { mask |= DayMask::SATURDAY; }
                                if data.alarm_daymask_sun { mask |= DayMask::SUNDAY; }

                                AlarmMode::Recurring(mask)
                            }
                            else {
                                AlarmMode::OneTime
                            };

                            let mut alarm = alarm.write().unwrap();
                            *alarm = Alarm::new(data.alarm_enabled,
                                                Time::from_str(&data.alarm_time),
                                                data.alarm_fade_length_s,
                                                (data.alarm_start_vol as f32)/100.0,
                                                (data.alarm_end_vol as f32)/100.0,
                                                mode);


                            // We just print what was received on stdout. Of course in a real application
                            // you probably want to process the data, eg. store it in a database.
                            println!("Received data: {:?}", data);

                            rouille::Response::redirect_303("/")
                        },

                        _ => rouille::Response::empty_404()
                )
            })
        });
    })
}

// The HTML document of the home page.
static TEMPLATE: &'static str = r#"
<html>
    <head>
        <title>WUMP WebUI</title>
        <script>
        function alarmModeSelected()\{
            var elem = document.getElementById("alarm_mode_recurring");
            var recurring_checked = elem.checked;

            var daymask_elems = document.getElementsByClassName("alarm_daymask");
            for(i = 0; i < daymask_elems.length; i++) \{
                daymask_elems[i].disabled = !recurring_checked;
            }
        }
        </script>
        <style>
        input[type="number"] \{
            width: 70px;
        }
        </style>
    </head>
    <body>
        <h1>WUMP WebUI</h1>
        <form action="" method="POST" enctype="multipart/form-data">
        <h2>Alarm</h2>
            <p><label><input id="alarm_enabled" type="checkbox" name="alarm_enabled" {alarm_enabled_checked}> Enabled</label></p>
        <h3>Time</h3>
        <p> Start time: <input type="time" name="alarm_time" value="{alarm_time}"></p>
        <p>
            <label><input id="alarm_mode_onetime" type="radio" name="alarm_mode" onchange="alarmModeSelected();" value="onetime" {alarm_mode_onetime_checked}> Onetime</label>
            <label><input id="alarm_mode_recurring" type="radio" name="alarm_mode" onchange="alarmModeSelected();" value="recurring" {alarm_mode_recurring_checked}> Recurring</label>
        </p>
        <p>
            <label><input class="alarm_daymask" type="checkbox" name="alarm_daymask_mon" {alarm_daymask_disabled} {alarm_daymask_mon_checked}> Mon</label>
            <label><input class="alarm_daymask" type="checkbox" name="alarm_daymask_tue" {alarm_daymask_disabled} {alarm_daymask_tue_checked}> Tue</label>
            <label><input class="alarm_daymask" type="checkbox" name="alarm_daymask_wed" {alarm_daymask_disabled} {alarm_daymask_wed_checked}> Wed</label>
            <label><input class="alarm_daymask" type="checkbox" name="alarm_daymask_thu" {alarm_daymask_disabled} {alarm_daymask_thu_checked}> Thu</label>
            <label><input class="alarm_daymask" type="checkbox" name="alarm_daymask_fri" {alarm_daymask_disabled} {alarm_daymask_fri_checked}> Fri</label>
            <label><input class="alarm_daymask" type="checkbox" name="alarm_daymask_sat" {alarm_daymask_disabled} {alarm_daymask_sat_checked}> Sat</label>
            <label><input class="alarm_daymask" type="checkbox" name="alarm_daymask_sun" {alarm_daymask_disabled} {alarm_daymask_sun_checked}> Sun</label>
        </p>
        <h3>Fade in</h3>
            <table>
                <tr><td align="left"> Length of fade (seconds):</td> <td align="left"><input type="number" step="1" min="0" name="alarm_fade_length_s" value="{alarm_fade_length_s}"></td></tr>
                <tr><td align="left"> Start volume (percentage):</td> <td align="left"><input type="number" step="1" min="0" max="100" name="alarm_start_vol" value="{alarm_start_vol}"></td></tr>
                <tr><td align="left"> End volume (percentage):</td> <td align="left"><input type="number" step="1" min="0" max="100" name="alarm_end_vol" value="{alarm_end_vol}"></td></tr>
            </table>
            <p><button>Save</button></p>
        </form>
    </body>
</html>
"#;
