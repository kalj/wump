extern crate sysfs_gpio;
extern crate retry;

use self::sysfs_gpio::{Direction, Pin, Edge};
use self::retry::{retry,delay};
use std::sync::mpsc;
use std::time::Duration;
use std::thread;

fn set_pin_dir(pin: & Pin, dir: Direction) -> std::result::Result<(), retry::Error<sysfs_gpio::Error>>
{
    retry(delay::Fixed::from_millis(100).take(3), || pin.set_direction(dir))
}

pub struct ButtonHandler {
    rx: mpsc::Receiver<u64>,
    thread_handles: Vec<thread::JoinHandle<()>>
}

impl ButtonHandler {
    pub fn new(buttons: &[u64]) -> ButtonHandler
    {
        let (tx, rx) = mpsc::channel();
        // let mut mute_state : bool = false;
        // let mute_pin = Pin::new(MUTE_PIN);
        // mute_pin.export().expect("Failed exporting mute pin");
        // set_pin_dir(&mute_pin, Direction::High).expect("Failed setting direction of mute pin");

        // let mut poff_state : bool = false;
        // let poff_pin = Pin::new(POFF_PIN);
        // poff_pin.export().expect("Failed exporting power off pin");
        // set_pin_dir(&poff_pin, Direction::High).expect("Failed setting direction of power off pin");

        let button_pins : Vec<(u64,Pin)> = buttons.iter().map(|&b| {
            let button = Pin::new(b);
            button.export().expect("Failed exporting pin");
            set_pin_dir(&button, Direction::In).expect("Failed setting direction of pin");
            button.set_edge(Edge::RisingEdge).expect("Failed setting edge of pin");
            (b,button)
        }).collect();

        // start up threads
        let thread_handles : Vec<thread::JoinHandle<_>> = button_pins.iter().map(|&(b,but)| {
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

        ButtonHandler { rx : rx,
                        thread_handles : thread_handles }
    }

    pub fn handle_events(&mut self, mut callback: impl FnMut(u64) ) {

        for x in self.rx.try_iter() {
            callback(x)
        }
    }
}
