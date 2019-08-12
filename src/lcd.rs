extern crate sysfs_gpio;
extern crate retry;

use self::sysfs_gpio::{Direction, Pin};
use self::retry::{retry,delay};
use std::time::Duration;
use std::thread::sleep;
use std::cmp;

// Define some device constants
pub const WIDTH: usize = 16;    // Maximum characters per line
const LCD_CHR: u8 = 1;
const LCD_CMD: u8 = 0;

pub const LINE_1: u8 = 0x80; // LCD RAM address for the 1st line
pub const LINE_2: u8 = 0xC0; // LCD RAM address for the 2nd line

// Timing constants
const E_PULSE: u32 = 500_000;
const E_DELAY: u32 = 500_000;

const CODE_CHAR_AA_U: u8 = 0x00;
const CODE_CHAR_AE_U: u8 = 0x01;
const CODE_CHAR_OE_U: u8 = 0x02;
const CODE_CHAR_AA_L: u8 = 0x03;
const CODE_CHAR_AE_L: u8 = 0xe1;
const CODE_CHAR_OE_L: u8 = 0xef;
const CODE_CHAR_UNKNOWN: u8 = 0xff;

pub struct Lcd
{
    e_pin: Pin,
    rs_pin: Pin,
    d_pins: [Pin;4],
}

fn decode(s: &str) ->Vec<u8>{

    s.chars().map(|c|
                  if c.len_utf8() == 1 {
                      c as u8
                  }
                  else {
                      match c {
                          'å' => CODE_CHAR_AA_L,
                          'ä' => CODE_CHAR_AE_L,
                          'ö' => CODE_CHAR_OE_L,
                          'Å' => CODE_CHAR_AA_U,
                          'Ä' => CODE_CHAR_AE_U,
                          'Ö' => CODE_CHAR_OE_U,
                          _ => CODE_CHAR_UNKNOWN,
                      }
                  }
    ).collect()
}

fn set_pin_dir(pin: & Pin, dir: Direction) -> std::result::Result<(), retry::Error<sysfs_gpio::Error>>
{
    retry(delay::Fixed::from_millis(100).take(3), || pin.set_direction(dir))
}

impl Lcd {
    pub fn new(e_pin_nr: u64, rs_pin_nr: u64, d4_pin_nr: u64,
               d5_pin_nr: u64, d6_pin_nr: u64, d7_pin_nr: u64) -> Lcd
    {
        let ep = Pin::new(e_pin_nr);
        ep.export().expect("Failed exporting E pin");
        set_pin_dir(&ep, Direction::Out).expect("Failed setting direction of E pin");

        let rsp = Pin::new(rs_pin_nr);
        rsp.export().expect("Failed exporting RS pin");
        set_pin_dir(&rsp, Direction::Out).expect("Failed setting direction of RS pin");

        let dps = [ Pin::new(d4_pin_nr),
                    Pin::new(d5_pin_nr),
                    Pin::new(d6_pin_nr),
                    Pin::new(d7_pin_nr)];

        for p in &dps {
            p.export().expect("Failed exporting data pin");
            set_pin_dir(&p, Direction::Out).expect("Failed setting direction of data pin");
        }

        Lcd {
            e_pin: ep,
            rs_pin: rsp,
            d_pins: dps,
        }
    }

    pub fn init(&self) {
        // Initialise display
        self.send_byte(0x33,LCD_CMD); // 110011 Initialise
        self.send_byte(0x32,LCD_CMD); // 110010 Initialise
        self.send_byte(0x06,LCD_CMD); // 000110 Cursor move direction
        self.send_byte(0x0C,LCD_CMD); // 001100 Display On,Cursor Off, Blink Off
        self.send_byte(0x28,LCD_CMD); // 101000 Data length, number of lines, font size
        self.send_byte(0x01,LCD_CMD); // 000001 Clear display
        sleep(Duration::new(0,E_DELAY));

        // ö exists, as 0xef
        // ä exists, as 0xe1

        // create custom character 'Å' as 0x00
        self.send_byte(0x40,LCD_CMD);
        self.send_byte(0x04,LCD_CHR);
        self.send_byte(0x0A,LCD_CHR);
        self.send_byte(0x04,LCD_CHR);
        self.send_byte(0x0A,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x1F,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);

        // create custom character 'Ä' as 0x01
        self.send_byte(0x48,LCD_CMD);
        self.send_byte(0x0A,LCD_CHR);
        self.send_byte(0x00,LCD_CHR);
        self.send_byte(0x04,LCD_CHR);
        self.send_byte(0x0A,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x1F,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);

        // create custom character 'Ö' as 0x02
        self.send_byte(0x50,LCD_CMD);
        self.send_byte(0x0A,LCD_CHR);
        self.send_byte(0x00,LCD_CHR);
        self.send_byte(0x0E,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x0E,LCD_CHR);

        // create custom character 'å' as 0x03
        self.send_byte(0x58,LCD_CMD);
        self.send_byte(0x04,LCD_CHR);
        self.send_byte(0x0A,LCD_CHR);
        self.send_byte(0x04,LCD_CHR);
        self.send_byte(0x0E,LCD_CHR);
        self.send_byte(0x01,LCD_CHR);
        self.send_byte(0x0F,LCD_CHR);
        self.send_byte(0x11,LCD_CHR);
        self.send_byte(0x0F,LCD_CHR);
    }

    fn send_byte(&self, bits: u8, mode: u8) {
        // Send byte to data pins
        // bits = data
        // mode = True  for character
        //        False for command

        self.rs_pin.set_value(mode).expect("Failed setting value of RS pin");

        // High bits
        for (i,p) in self.d_pins.iter().enumerate() {
            p.set_value( (bits>>(4+i)) & 0x01).expect("Failed setting value of data pin");
        }

        // Toggle 'Enable' pin
        self.toggle_enable();

        // Low bits
        for (i,p) in self.d_pins.iter().enumerate() {
            p.set_value( (bits >> i) & 0x01).expect("Failed setting value of pin");
        }

        // Toggle 'Enable' pin
        self.toggle_enable();
    }

    fn toggle_enable(&self) {
        // Toggle enable
        sleep(Duration::new(0,E_DELAY));
        self.e_pin.set_value(1).expect("Failed setting value of E pin");
        sleep(Duration::new(0,E_PULSE));
        self.e_pin.set_value(0).expect("Failed setting value of E pin");
        sleep(Duration::new(0,E_DELAY));
    }


    pub fn print_string(&self, message: & str, line: u8) {
        // Decode and pass
        self.print_bytestr(&decode(message),line);
    }

    fn print_bytestr(&self, bytes: &[u8], line: u8) {
        // Send string to display

        self.send_byte(line, LCD_CMD);

        let n = cmp::min(WIDTH,bytes.len());

        let mut i=0;

        // write actual string
        while i<n {
            let c = bytes[i];
            self.send_byte(c, LCD_CHR);
            i+=1;
        }

        // write padding
        while i<WIDTH {
            self.send_byte(' ' as u8, LCD_CHR);
            i+=1;
        }
    }
}

impl Drop for Lcd
{
    fn drop(&mut self)
    {
        self.e_pin.unexport().expect("Failed unexporting E pin");
        self.rs_pin.unexport().expect("Failed unexporting RS pin");
        for (i, p) in self.d_pins.iter().enumerate() {
            println!("Unexporting data pin {}",i);
            p.unexport().expect("Failed unexporting data pin");
        }
    }
}
