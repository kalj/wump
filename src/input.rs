extern crate rppal;

use self::rppal::gpio::{Gpio, Trigger, InputPin, Level};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum InputEvent {
    Button(u8),
    RotaryEncoder(i8),
}

pub struct InputHandler {
    rx: mpsc::Receiver<InputEvent>,
    _button_pins: Vec<InputPin>,
    _rotary_encoder_thread: thread::JoinHandle<()>,
}

impl InputHandler {
    pub fn new(button_pin_ids: &[u8], rotary_encoder_pin_ids: (u8, u8)) -> InputHandler
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
        let _button_pins: Vec<InputPin> =
            button_pin_ids.iter().map(|&b| {

                let mut button_pin = gpio.get(b).expect("Failed setting button gpio pin to input").into_input();
                let tx_b = tx.clone();
                button_pin.set_async_interrupt(Trigger::RisingEdge,
                                               move |_| {
                                                   tx_b.send(InputEvent::Button(b)).unwrap();
                                               }).unwrap();
                button_pin
            }).collect();

        let _rotary_encoder_thread = thread::spawn(move || {

            let tx_rotenc = tx.clone();
            let rotenc_a_pin = gpio.get(rotary_encoder_pin_ids.0).expect("Failed setting rotary encoder a gpio pin to input").into_input();
            let rotenc_b_pin = gpio.get(rotary_encoder_pin_ids.1).expect("Failed setting rotary encoder b gpio pin to input").into_input();

            let mut last_clk_state = Level::High;

            loop {
                let aval = rotenc_a_pin.read();
                match aval {
                    Level::High => if last_clk_state == Level::Low {
                        if let Level::Low = rotenc_b_pin.read() {
                            tx_rotenc.send(InputEvent::RotaryEncoder(1)).unwrap();
                        } else {
                            tx_rotenc.send(InputEvent::RotaryEncoder(-1)).unwrap();
                        }
                    },
                    _  => {}
                }
                last_clk_state = aval;

                thread::sleep(Duration::from_millis(1));
            }
        });

        InputHandler { rx, _button_pins, _rotary_encoder_thread }
    }

    pub fn handle_events(&mut self, mut callback: impl FnMut(InputEvent) ) {

        for x in self.rx.try_iter() {
            callback(x)
        }
    }
}
