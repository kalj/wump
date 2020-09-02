extern crate rppal;

use self::rppal::gpio::{Gpio, Trigger, InputPin};
use std::sync::mpsc;

pub struct ButtonHandler {
    rx: mpsc::Receiver<u8>,
    button_pins: Vec<InputPin>
}

impl ButtonHandler {
    pub fn new(buttons: &[u8]) -> ButtonHandler
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

        let gpio = Gpio::new().unwrap();
        let button_pins: Vec<InputPin> =
            buttons.iter().map(|&b| {

                let mut button_pin = gpio.get(b).expect("Failed setting button gpio pin to input").into_input();
                let tx_b = tx.clone();
                button_pin.set_async_interrupt(Trigger::RisingEdge,
                                               move |_| {
                                                   tx_b.send(b);
                                               });
                button_pin
            }).collect();

        ButtonHandler { rx, button_pins }
    }

    pub fn handle_events(&mut self, mut callback: impl FnMut(u8) ) {

        for x in self.rx.try_iter() {
            callback(x)
        }
    }
}
