extern crate i2cdev;

use self::i2cdev::core::*;
use self::i2cdev::linux::{LinuxI2CDevice};
use std::time::Duration;
use std::thread::sleep;
use std::cmp;
use std::default::{Default};

// Define some device constants
pub const WIDTH: usize = 16;    // Maximum characters per line
const LCD_CHR: u8 = 1;
const LCD_CMD: u8 = 0;

const BL_BIT: u8 = 0x08;
const RS_BIT: u8 = 0b1;
const EN_BIT: u8 = 0b100;

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

fn fill_buf(buf: &mut [u8;WIDTH], vec: &[u8])
{
    let n = cmp::min(WIDTH,vec.len());

    // write actual string
    for i in 0..n {
        buf[i] = vec[i];
    }

    // write padding
    for i in n..WIDTH {
        buf[i] = ' ' as u8;
    }
}

struct LcdDev
{
    dev: LinuxI2CDevice,
    backlight: bool
}

impl LcdDev {
    pub fn new(path: &str, addr: u16, bl: bool) -> LcdDev
    {
        let dev = LinuxI2CDevice::new(path, addr).expect("Failed setting up I2C device for LCD.");
        LcdDev {
            dev: dev,
            backlight: bl
        }
    }

    pub fn init(&mut self) {
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

    fn get_backlight(&self) -> bool {
        self.backlight
    }

    fn set_backlight(&mut self, bl: bool) {
        self.backlight = bl
    }

    fn send_byte(&mut self, bits: u8, mode: u8) {
        // Send byte to data pins
        // bits = data
        // mode = True  for character
        //        False for command

        let bl: u8 = if self.backlight {BL_BIT} else {0};
        const DATA_BITS: u8 = 0xF0;

        // High bits
        let data = (bits & DATA_BITS) | (mode & RS_BIT) | bl;

        self.dev.smbus_write_byte(data).unwrap();
        sleep(Duration::new(0,E_DELAY));

        self.dev.smbus_write_byte(data | EN_BIT).unwrap();
        sleep(Duration::new(0,E_PULSE));

        self.dev.smbus_write_byte(data).unwrap(); // & (~0x4)
        sleep(Duration::new(0,E_DELAY));


        // Low bits
        let data = ((bits<<4) & DATA_BITS) | (mode & RS_BIT) | bl;

        self.dev.smbus_write_byte(data).unwrap();
        sleep(Duration::new(0,E_DELAY));

        self.dev.smbus_write_byte(data | EN_BIT).unwrap();
        sleep(Duration::new(0,E_PULSE));

        self.dev.smbus_write_byte(data).unwrap(); // & (~0x4)
        sleep(Duration::new(0,E_DELAY));
    }

    fn print_buf(&mut self, buf: [u8;WIDTH], line: u8) {
        // Send string to display
        self.send_byte(line, LCD_CMD);

        for i in 0..WIDTH {
            self.send_byte(buf[i], LCD_CHR);
        }
    }

    fn print_bytestr(&mut self, bytes: &[u8], line: u8) {
        let mut buf: [u8;WIDTH] = [0;WIDTH];
        fill_buf(&mut buf, bytes);
        self.print_buf(buf, line);
    }

    fn print_string(&mut self, message: & str, line: u8) {
        // Decode and pass
        self.print_bytestr(&decode(message),line);
    }
}


pub struct Lcd
{
    dev: LcdDev,
    lines: [String;2]
}

impl Lcd {
    pub fn new(path: &str, addr: u16) -> Lcd
    {
        let dev = LcdDev::new(path, addr, true);
        Lcd {
            dev: dev,
            lines: Default::default()
        }
    }

    pub fn init(&mut self) {
        self.dev.init();
    }

    fn update(&mut self)
    {
        self.dev.print_bytestr(&decode(&self.lines[0]),LINE_1);
        self.dev.print_bytestr(&decode(&self.lines[1]),LINE_2);
    }


    pub fn set_lines(&mut self, line1: &str, line2: &str)
    {
        if self.lines[0] != line1 || self.lines[1] != line2 {
            self.lines[0] = line1.to_string();
            self.lines[1] = line2.to_string();
            self.update();
        }
    }

    pub fn set_backlight(&mut self, newbl: bool)
    {
        let bl = self.dev.get_backlight();
        if bl != newbl {
            self.dev.set_backlight(newbl);
            self.update();
        }
    }

    pub fn toggle_backlight(&mut self)
    {
        let bl = self.dev.get_backlight();
        self.dev.set_backlight(!bl);
        self.update();
    }
}
