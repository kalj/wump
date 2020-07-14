extern crate rppal;
extern crate ssd1322;

use self::ssd1322 as oled;
use self::rppal::gpio::{Gpio, OutputPin};
use self::rppal::spi::{Bus, Mode, SlaveSelect, Spi};

use crate::fontmap::FontBitmapSet;

use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use chrono::{DateTime, Local};

use std::iter;
type Display = oled::Display<oled::SpiInterface<Spi, OutputPin>>;

struct TextCanvas {
    bmpset: FontBitmapSet,
    upper_left: [u32; 2],
    lower_right: [u32; 2]
}

struct TextCanvasIterator<'a> {
    canvas: &'a TextCanvas,
    text: &'a [char],
    left_padding: u32,
    row: u32,
    col: u32,
}

impl TextCanvasIterator<'_> {
    fn new<'a>(canvas: &'a TextCanvas, text: &'a [char]) -> TextCanvasIterator<'a> {

        let text_width = (text.len() as u32)*canvas.bmpset.glyph_width();

        if text_width > canvas.width() {
            panic!("Text is too long to fit in canvas width");
        }

        let left_padding = (canvas.width() - text_width)/2;
        TextCanvasIterator { canvas, text, left_padding, row: 0, col: 0 }
    }
}

impl Iterator for TextCanvasIterator<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {

        if self.row >= self.canvas.height() {
            return None;
        }

        let mut val = 0;

        let char_width = self.canvas.bmpset.glyph_width();

        if self.col >= self.left_padding && self.col < (self.left_padding+(self.text.len() as u32)*char_width){
            let net_col = self.col - self.left_padding;

            let char_idx = net_col/char_width;
            let col_in_char = net_col%char_width;

            let c = self.text[char_idx as usize];
            val = self.canvas.bmpset.get(c, self.row, col_in_char);
        }

        self.col+=1;
        if self.col>=self.canvas.width() {
            self.col=0;
            self.row+=1;
        }

        return Some(val);
    }
}

impl TextCanvas {
    fn new(font_data: &[u8], upper_left: [u32; 2], lower_right: [u32; 2]) -> TextCanvas {

        if upper_left[0] >= lower_right[0] || upper_left[1] >= lower_right[1] {
            panic!("`upper_left` must be strictly to the left and above `lower_right`");
        }

        let font_size = lower_right[1]-upper_left[1];

        let chars: HashSet<_> = "0123456789:".chars().collect();
        let bmpset = FontBitmapSet::new_with_charset(font_data, font_size, &chars);
        TextCanvas { bmpset, upper_left, lower_right }
    }

    fn width(&self) -> u32 {
        self.lower_right[0] - self.upper_left[0]
    }

    fn height(&self) -> u32 {
        self.lower_right[1] - self.upper_left[1]
    }

    fn render_text(&self, dpy: &mut Display, text: &str) {

        let ul = oled::PixelCoord(self.upper_left[0] as i16, self.upper_left[1] as i16);
        let lr = oled::PixelCoord(self.lower_right[0] as i16, self.lower_right[1] as i16);

        let chars: Vec<char> = text.chars().collect();
        let pixel_iter = TextCanvasIterator::new(self, &chars);

        dpy.region(ul, lr).unwrap().draw(pixel_iter).unwrap();
    }
}

pub struct Oled
{
    rst_pin: OutputPin,
    dpy: Display,
    clock_canvas: TextCanvas
}

impl Oled {
    pub fn new(dc_pin_id: u8, rst_pin_id: u8, spi_bus: Bus, spi_ss: SlaveSelect) -> Oled
    {
        let gpio = Gpio::new().unwrap();
        let dc_pin = gpio.get(dc_pin_id).unwrap().into_output();
        let rst_pin = gpio.get(rst_pin_id).unwrap().into_output();

        let spi = Spi::new(spi_bus, spi_ss, 10_000_000, Mode::Mode0).unwrap();

        // Create the SpiInterface and Display.
        let dpy = oled::Display::new(
            oled::SpiInterface::new(spi, dc_pin),
            oled::PixelCoord(256, 64),
            oled::PixelCoord(112, 0),
        );

        let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");

        let upper_left = [0, 0];
        let lower_right = [256, 64];
        let clock_canvas = TextCanvas::new(font_data, upper_left, lower_right);

        Oled { rst_pin, dpy, clock_canvas }
    }

    pub fn init(&mut self) {

        // Assert the display's /RESET for 10ms.
        self.rst_pin.set_low();
        thread::sleep(Duration::from_millis(100));
        self.rst_pin.set_high();
        thread::sleep(Duration::from_millis(400));

        // Initialize the display. These parameters are taken from the Newhaven datasheet for the
        // NHD-3.12-25664UCY2.
        self.dpy.init(
            oled::Config::new(
                oled::ComScanDirection::RowZeroLast,
                oled::ComLayout::Progressive,
            )
                .clock_fosc_divset(9, 1)
                .display_enhancements(true, true)
                .contrast_current(59)
                .phase_lengths(5, 14)
                .precharge_voltage(31)
                .second_precharge_period(8)
                .com_deselect_voltage(7),
        ) .unwrap();

        // clear
        {
            let mut region = self.dpy
                .region(oled::PixelCoord(0, 0), oled::PixelCoord(256, 64))
                .unwrap();
            region.draw(iter::repeat(0)).unwrap();
        }

    }

    pub fn show_time(&mut self, now: &DateTime<Local>)
    {
        let text = now.format("%H:%M").to_string();

        self.clock_canvas.render_text(&mut self.dpy, &text);
    }
}
