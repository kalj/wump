extern crate sysfs_gpio;
extern crate retry;

use self::sysfs_gpio::{Direction, Pin};
use self::retry::{retry,delay};
use std::time::Duration;
use std::thread::sleep;
use std::cmp;

// Define some device constants
pub const LCD_WIDTH: usize = 16;    // Maximum characters per line
const LCD_CHR: u8 = 1;
const LCD_CMD: u8 = 0;

pub const LCD_LINE_1: u8 = 0x80; // LCD RAM address for the 1st line
pub const LCD_LINE_2: u8 = 0xC0; // LCD RAM address for the 2nd line

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

pub struct Pins
{
    e_pin: Pin,
    rs_pin: Pin,
    d_pins: [Pin;4],
}

pub fn lcd_init(pins: & Pins) {
    // Initialise display
    lcd_byte(pins,0x33,LCD_CMD); // 110011 Initialise
    lcd_byte(pins,0x32,LCD_CMD); // 110010 Initialise
    lcd_byte(pins,0x06,LCD_CMD); // 000110 Cursor move direction
    lcd_byte(pins,0x0C,LCD_CMD); // 001100 Display On,Cursor Off, Blink Off
    lcd_byte(pins,0x28,LCD_CMD); // 101000 Data length, number of lines, font size
    lcd_byte(pins,0x01,LCD_CMD); // 000001 Clear display
    sleep(Duration::new(0,E_DELAY));

    // ö exists, as 0xef
    // ä exists, as 0xe1

    // create custom character 'Å' as 0x00
    lcd_byte(pins,0x40,LCD_CMD);
    lcd_byte(pins,0x04,LCD_CHR);
    lcd_byte(pins,0x0A,LCD_CHR);
    lcd_byte(pins,0x04,LCD_CHR);
    lcd_byte(pins,0x0A,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x1F,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);

    // create custom character 'Ä' as 0x01
    lcd_byte(pins,0x48,LCD_CMD);
    lcd_byte(pins,0x0A,LCD_CHR);
    lcd_byte(pins,0x00,LCD_CHR);
    lcd_byte(pins,0x04,LCD_CHR);
    lcd_byte(pins,0x0A,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x1F,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);

    // create custom character 'Ö' as 0x02
    lcd_byte(pins,0x50,LCD_CMD);
    lcd_byte(pins,0x0A,LCD_CHR);
    lcd_byte(pins,0x00,LCD_CHR);
    lcd_byte(pins,0x0E,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x0E,LCD_CHR);

    // create custom character 'å' as 0x03
    lcd_byte(pins,0x58,LCD_CMD);
    lcd_byte(pins,0x04,LCD_CHR);
    lcd_byte(pins,0x0A,LCD_CHR);
    lcd_byte(pins,0x04,LCD_CHR);
    lcd_byte(pins,0x0E,LCD_CHR);
    lcd_byte(pins,0x01,LCD_CHR);
    lcd_byte(pins,0x0F,LCD_CHR);
    lcd_byte(pins,0x11,LCD_CHR);
    lcd_byte(pins,0x0F,LCD_CHR);
}

fn lcd_byte(pins: & Pins, bits: u8, mode: u8) {
    // Send byte to data pins
    // bits = data
    // mode = True  for character
    //        False for command

    pins.rs_pin.set_value(mode).expect("Failed setting value of RS pin");

    // High bits
    for (i,p) in pins.d_pins.iter().enumerate() {
        p.set_value( (bits>>(4+i)) & 0x01).expect("Failed setting value of data pin");
    }

    // Toggle 'Enable' pin
    lcd_toggle_enable(pins);

    // Low bits
    for (i,p) in pins.d_pins.iter().enumerate() {
        p.set_value( (bits >> i) & 0x01).expect("Failed setting value of pin");
    }

    // Toggle 'Enable' pin
    lcd_toggle_enable(pins);
}

fn lcd_toggle_enable(pins: & Pins) {
    // Toggle enable
    sleep(Duration::new(0,E_DELAY));
    pins.e_pin.set_value(1).expect("Failed setting value of E pin");
    sleep(Duration::new(0,E_PULSE));
    pins.e_pin.set_value(0).expect("Failed setting value of E pin");
    sleep(Duration::new(0,E_DELAY));
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

pub fn lcd_string(pins: & Pins, message: & str, line: u8) {
    // Decode and pass
    lcd_bytestr(pins,&decode(message),line);
}

fn lcd_bytestr(pins: & Pins, bytes: &[u8], line: u8) {
    // Send string to display

    lcd_byte(&pins, line, LCD_CMD);

    let n = cmp::min(LCD_WIDTH,bytes.len());

    let mut i=0;

    // write actual string
    while i<n {
        let c = bytes[i];
        lcd_byte(&pins, c, LCD_CHR);
        i+=1;
    }

    // write padding
    while i<LCD_WIDTH {
        lcd_byte(&pins, ' ' as u8, LCD_CHR);
        i+=1;
    }

}

fn set_pin_dir(pin: & Pin, dir: Direction) -> std::result::Result<(), retry::Error<sysfs_gpio::Error>>
{
    retry(delay::Fixed::from_millis(100).take(3), || pin.set_direction(dir))
}

pub fn create_pins(e_pin_nr: u64, rs_pin_nr: u64, d4_pin_nr: u64,
               d5_pin_nr: u64, d6_pin_nr: u64, d7_pin_nr: u64) -> Pins
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

    Pins {
        e_pin: ep,
        rs_pin: rsp,
        d_pins: dps,
    }
}

pub fn destroy_pins(pins: &Pins)
{
    pins.e_pin.unexport().expect("Failed unexporting E pin");
    pins.rs_pin.unexport().expect("Failed unexporting RS pin");

    for p in &pins.d_pins {
        p.unexport().expect("Failed unexporting data pin");
    }
}
