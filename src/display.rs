extern crate rppal;
extern crate spidev;
extern crate bitmap_font;

use self::spidev::{Spidev, SpidevOptions, SpiModeFlags, SpidevTransfer};
use self::rppal::gpio::{OutputPin, Gpio};

use std::io::Write;
use std::io;
use std::thread;
use std::time::Duration;
use std::cmp::min;
use chrono::{DateTime, Local};


const GPIO_PIN_MCP2317_CS:  u8 = 7;  // == Pin 26 == CE1
const GPIO_PIN_MCP2317_RST: u8 = 23; // == Pin 16 ==

const E_BIT:   u8 = 0;
const DI_BIT:  u8 = 1;
const RW_BIT:  u8 = 2;
const RST_BIT: u8 = 3;
const CS1_BIT: u8 = 4;
const CS2_BIT: u8 = 5;
const BL_BIT:  u8 = 7;

const DI_D:    u8 = 1;
const DI_I:    u8 = 0;
const RW_R:    u8 = 1;
const RW_W:    u8 = 0;
const CS_EN:   u8 = 1;
const CS_DIS:  u8 = 0;
const RST_ON:  u8 = 0;
const RST_OFF: u8 = 1;

const MCP23S17_READCMD:   u8 = 0x41;
const MCP23S17_WRITECMD:  u8 = 0x40;
const MCP23S17_IODIRA:    u8 = 0x00;
const MCP23S17_IODIRB:    u8 = 0x01;
const MCP23S17_IOCON:     u8 = 0x05;
const MCP23S17_GPIOA:     u8 = 0x12;
const MCP23S17_GPIOB:     u8 = 0x13;
const MCP23S17_OLATA:     u8 = 0x14;
const MCP23S17_OLATB:     u8 = 0x15;

struct MCP23S17 {
    spi:     Spidev,
    _cs_pin: OutputPin,
    rst_pin: OutputPin,
}

impl MCP23S17 {
    fn new() -> io::Result<MCP23S17> {
        let gpio = Gpio::new().unwrap();

        let mut cs_pin = gpio.get(GPIO_PIN_MCP2317_CS).unwrap().into_output();
        cs_pin.set_high();

        let mut rst_pin = gpio.get(GPIO_PIN_MCP2317_RST).unwrap().into_output();
        rst_pin.set_high();

        let mut spi = Spidev::open("/dev/spidev0.1")?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(10_000_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options)?;

        Ok(MCP23S17 { spi, _cs_pin: cs_pin, rst_pin })
    }

    fn reset(&mut self) {
        self.rst_pin.set_low();
        thread::sleep(Duration::from_micros(1));
        self.rst_pin.set_high();
    }

    fn write_reg(&mut self, addr: u8, val: u8) -> io::Result<()> {
        let n_written = self.spi.write(&[MCP23S17_WRITECMD, addr, val])?;
        if n_written != 3 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Wrong number of bytes written ({} instead of {})", n_written, 3),
            ));
        }
        Ok(())
    }

    fn write_reg_rep(&mut self, addr: u8, val: &[u8]) -> io::Result<()> {
        let mut tx = vec![MCP23S17_WRITECMD, addr];
        tx.extend_from_slice(val);
        let n_written = self.spi.write(&tx)?;
        if n_written != tx.len() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Wrong number of bytes written ({} instead of {})", n_written, tx.len()),
            ));
        }
        Ok(())
    }

    fn read_reg(&mut self, addr: u8) -> io::Result<u8> {
        let tx_buf = [MCP23S17_READCMD, addr, 0x00];
        let mut rx_buf = [0_u8; 3];
        {
            let mut transfer = SpidevTransfer::read_write(&tx_buf, &mut rx_buf);
            self.spi.transfer(&mut transfer)?;
        }

        Ok(rx_buf[2])
    }
}

struct NT7108 {
    iface:     MCP23S17,
    ctrl_bits: u8,
}

enum Direction {
    Input,
    Output,
}

#[derive(Copy, Clone)]
enum ChipId {
    Chip1,
    Chip2,
}

impl NT7108 {
    pub fn new() -> io::Result<NT7108> {
        let mut iface = MCP23S17::new()?;

        iface.reset();

        iface.write_reg(MCP23S17_IOCON, 0x20)?; // disable sequential operation (auto inc of addr)

        iface.write_reg(MCP23S17_IODIRA, 0xff)?;

        // set default value of output pins
        let ctrl_bits = (1 << E_BIT)
            | (DI_D << DI_BIT)
            | (RW_R << RW_BIT)
            | (RST_OFF << RST_BIT)
            | (CS_DIS << CS1_BIT)
            | (CS_DIS << CS2_BIT)
            | (0 << BL_BIT);
        iface.write_reg(MCP23S17_OLATB, ctrl_bits)?;
        iface.write_reg(MCP23S17_IODIRB, 0x00)?; // all output

        let mut dev = NT7108 { iface, ctrl_bits };

        // perform reset
        dev.update_ctrl_bits(1 << RST_BIT, RST_ON << RST_BIT)?;
        thread::sleep(Duration::from_micros(1));
        dev.update_ctrl_bits(1 << RST_BIT, RST_OFF << RST_BIT)?;
        thread::sleep(Duration::from_micros(1));

        Ok(dev)
    }

    fn read_bus(&mut self) -> io::Result<u8> {
        self.iface.read_reg(MCP23S17_GPIOA)
    }

    fn write_bus(&mut self, data: u8) -> io::Result<()> {
        self.iface.write_reg(MCP23S17_GPIOA, data)
    }

    fn set_busdir(&mut self, dir: Direction) -> io::Result<()> {
        let mask = match dir {
            Direction::Output => 0x00u8,
            Direction::Input => 0xffu8,
        };
        self.iface.write_reg(MCP23S17_IODIRA, mask)
    }

    fn update_ctrl_bits(&mut self, mask: u8, new_bits: u8) -> io::Result<()> {
        let cur_bits = self.ctrl_bits & mask;

        if new_bits != cur_bits {
            // set bits to new value
            self.ctrl_bits = (self.ctrl_bits & !mask) | new_bits;
            // write new state
            self.iface.write_reg(MCP23S17_GPIOB, self.ctrl_bits)
        } else {
            Ok(())
        }
    }

    fn enable_chip(&mut self, id: ChipId) -> io::Result<()> {
        let mut cs1 = CS_DIS;
        let mut cs2 = CS_DIS;

        match id {
            ChipId::Chip1 => {
                cs1 = CS_EN;
            }
            ChipId::Chip2 => {
                cs2 = CS_EN;
            }
        }

        self.update_ctrl_bits((1 << CS2_BIT) | (1 << CS1_BIT), (cs2 << CS2_BIT) | (cs1 << CS1_BIT))
    }

    fn disable_chips(&mut self) -> io::Result<()> {
        self.update_ctrl_bits((1 << CS2_BIT) | (1 << CS1_BIT), (CS_DIS << CS2_BIT) | (CS_DIS << CS1_BIT))
    }

    fn write(&mut self, chip: ChipId, is_data: bool, b: u8) -> io::Result<()> {
        self.update_ctrl_bits(
            (1 << DI_BIT) | (1 << RW_BIT),
            ((if is_data { DI_D } else { DI_I }) << DI_BIT) | (RW_W << RW_BIT),
        )?;

        self.set_busdir(Direction::Output)?;
        self.write_bus(b)?;

        self.enable_chip(chip)?;

        // normally high
        thread::sleep(Duration::from_micros(1));
        self.update_ctrl_bits(1 << E_BIT, 0 << E_BIT)?; // set E to low
        thread::sleep(Duration::from_micros(1));

        self.disable_chips()?;
        self.update_ctrl_bits(1 << E_BIT, 1 << E_BIT)?; // set E to high

        self.set_busdir(Direction::Input)?;
        self.update_ctrl_bits(1 << RW_BIT, RW_R << RW_BIT) // set back to read state
    }

    fn read(&mut self, chip: ChipId, is_data: bool) -> io::Result<u8> {
        self.set_busdir(Direction::Input)?;

        self.update_ctrl_bits(
            (1 << DI_BIT) | (1 << RW_BIT),
            ((if is_data { DI_D } else { DI_I }) << DI_BIT) | (RW_R << RW_BIT),
        )?;
        self.enable_chip(chip)?;

        self.update_ctrl_bits(1 << E_BIT, 1 << E_BIT)?; // set E to high

        thread::sleep(Duration::from_micros(1)); // random time
        let data = self.read_bus()?;

        self.disable_chips()?;

        Ok(data)
    }

    pub fn set_backlight(&mut self, on: bool) -> io::Result<()> {
        self.update_ctrl_bits(1 << BL_BIT, (if on { 1 } else { 0 }) << BL_BIT)
    }

    pub fn set_onoff(&mut self, chip: ChipId, on: bool) -> io::Result<()> {
        self.write(chip, false, 0x3e | if on { 1 } else { 0 })
    }

    pub fn set_addr(&mut self, chip: ChipId, yaddr: u8) -> io::Result<()> {
        self.write(chip, false, 0x40 | (0x3f & yaddr))
    }

    pub fn set_page(&mut self, chip: ChipId, xaddr: u8) -> io::Result<()> {
        self.write(chip, false, 0xb8 | (0x07 & xaddr))
    }

    pub fn set_startline(&mut self, chip: ChipId, startline: u8) -> io::Result<()> {
        self.write(chip, false, 0xc0 | (0x3f & startline))
    }

    pub fn write_data(&mut self, chip: ChipId, data: u8) -> io::Result<()> {
        self.write(chip, true, data)
    }

    pub fn read_status(&mut self, chip: ChipId) -> io::Result<u8> {
        self.read(chip, false)
    }

    pub fn read_data(&mut self, chip: ChipId) -> io::Result<u8> {
        self.read(chip, true)
    }
}

struct Lcd128x64 {
    dev:          NT7108,
    backlight_on: bool,
}

impl Lcd128x64 {
    pub fn new() -> io::Result<Lcd128x64> {
        let dev = NT7108::new()?;
        let backlight_on = false;
        Ok(Lcd128x64 { dev, backlight_on })
    }

    pub fn set_onoff(&mut self, on: bool) -> io::Result<()> {
        self.dev.set_onoff(ChipId::Chip1, on)?;
        self.dev.set_onoff(ChipId::Chip2, on)
    }

    pub fn get_backlight(&self) -> bool {
        self.backlight_on
    }

    pub fn set_backlight(&mut self, on: bool) -> io::Result<()> {
        if on != self.backlight_on {
            self.dev.set_backlight(on)?;
            self.backlight_on = on;
        }

        Ok(())
    }

    pub fn fill_byte(&mut self, byte: u8) -> io::Result<()> {
        for chip in [ChipId::Chip1, ChipId::Chip2].iter() {
            self.dev.set_startline(*chip, 0)?;
            self.dev.set_addr(*chip, 0)?;

            for p in 0..8 {
                self.dev.set_page(*chip, p)?;
                for _i in 0..64 {
                    self.dev.write_data(*chip, byte)?;
                }
            }
        }

        Ok(())
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.fill_byte(0x00)
    }

    pub fn set_bytes_at(&mut self, row: usize, col: usize, bytes: &[u8]) -> io::Result<usize> {
        let n_bytes = bytes.len();
        let mut n_written = 0;
        // for chip 1
        if col < 64 {
            let chip = ChipId::Chip1;
            let chip_n_bytes = min(n_bytes, 64 - col);

            self.dev.set_page(chip, row as u8)?;
            self.dev.set_addr(chip, col as u8)?;
            for byte in bytes.iter().take(chip_n_bytes) {
                self.dev.write_data(chip, *byte)?;
            }
            n_written = chip_n_bytes;
        }

        // for chip 1
        if n_bytes > n_written {
            let chip = ChipId::Chip2;
            let chip_col = (col + n_written) - 64;
            let chip_n_bytes = min(n_bytes - n_written, 64 - chip_col);

            self.dev.set_page(chip, row as u8)?;
            self.dev.set_addr(chip, chip_col as u8)?;

            for j in 0..chip_n_bytes {
                self.dev.write_data(chip, bytes[(n_written + j)])?;
            }

            n_written += chip_n_bytes;
        }

        Ok(n_written)
    }
}

#[derive(Copy, Clone)]
struct BufferEntry {
    val:   u8,
    dirty: bool,
}

const LCD_WIDTH: usize = 128;
const LCD_N_BYTE_ROWS: usize = 8;
const LCD_HEIGHT: usize = 8 * LCD_N_BYTE_ROWS;

struct BufferedLcd {
    dev:    Lcd128x64,
    buffer: [[BufferEntry; LCD_WIDTH]; LCD_N_BYTE_ROWS],
}

struct Vec2d {
    data: Vec<bool>,
    w:    usize,
}

impl Vec2d {
    fn new(h: usize, w: usize) -> Vec2d {
        Vec2d { data: vec![false; h * w], w }
    }

    fn width(&self) -> usize {
        self.w
    }

    fn height(&self) -> usize {
        self.data.len() / self.w
    }

    fn get(&self, i: usize, j: usize) -> bool {
        self.data[i * self.w + j]
    }

    fn set(&mut self, i: usize, j: usize, val: bool) {
        self.data[i * self.w + j] = val
    }
}

impl BufferedLcd {
    fn send_bytes(dev: &mut Lcd128x64, entries: &mut [BufferEntry], row: usize, col: usize) -> io::Result<()> {
        let bytes: Vec<u8> = entries.iter().map(|e| e.val).collect();
        let n_written = dev.set_bytes_at(row, col, &bytes)?;
        if n_written != entries.len() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Wrong number of bytes written ({} instead of {})", n_written, entries.len()),
            ));
        }
        for e in entries {
            e.dirty = false;
        }

        Ok(())
    }

    pub fn get_backlight(&mut self) -> bool {
        self.dev.get_backlight()
    }

    pub fn set_backlight(&mut self, on: bool) -> io::Result<()> {
        self.dev.set_backlight(on)
    }

    pub fn write_back(&mut self) -> io::Result<()> {
        for (ri, row) in self.buffer.iter_mut().enumerate() {
            // gather consecutive dirty bytes
            let mut ibeg_o = None;
            let mut iend = 0;

            for i in 0..row.len() {
                if row[i].dirty {
                    match ibeg_o {
                        None => {
                            ibeg_o = Some(i);
                            iend = i;
                        }
                        Some(_) => {
                            iend = i;
                        }
                    }
                } else if let Some(ibeg) = ibeg_o {
                    // end of consecutive list of dirty bytes, send them
                    BufferedLcd::send_bytes(&mut self.dev, &mut row[ibeg..=iend], ri, ibeg)?;

                    // reset indices
                    ibeg_o = None;
                    iend = 0; // not necessary
                }
            }

            // handle last chunk
            if let Some(ibeg) = ibeg_o {
                BufferedLcd::send_bytes(&mut self.dev, &mut row[ibeg..=iend], ri, ibeg)?;
            }
        }
        Ok(())
    }

    pub fn new() -> io::Result<BufferedLcd> {
        let mut dev = Lcd128x64::new()?;

        // Assert the display's /RESET for 10ms.
        dev.set_onoff(true)?;

        let buffer = [[BufferEntry { val: 0, dirty: true }; LCD_WIDTH]; LCD_N_BYTE_ROWS];
        let mut obj = BufferedLcd { dev, buffer };
        obj.write_back()?;
        Ok(obj)
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.dev.clear()
    }

    pub fn set_bits_at(&mut self, row: usize, col: usize, bits: &Vec2d, wb: bool) -> io::Result<()> {

        if (row+bits.height()) > LCD_HEIGHT || (col+bits.width()) >LCD_WIDTH {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      format!("Bit matrix is too large ({} x {}) to fit in buffer at this position ({},{}) for buffer size {}x{}",
                                              bits.height(), bits.width(), row, col, LCD_HEIGHT, LCD_WIDTH)));
        }

        // for (i,row_bits) in bits.iter().enumerate() {
        for i in 0..bits.height() {
            let rowi = row + i;

            let byte_row_index = rowi / 8;
            let bit_byte_index = rowi % 8;
            for j in 0..bits.width() {
                let colj = col + j;

                let mut entry = &mut self.buffer[byte_row_index][colj];
                let old_byte = entry.val;

                // set bit to new value
                if bits.get(i, j) {
                    entry.val |= 1 << bit_byte_index;
                } else {
                    entry.val &= !(1 << bit_byte_index);
                }

                // set dirty if changed
                if entry.val != old_byte {
                    entry.dirty = true;
                }
            }
        }

        if wb {
            self.write_back()?;
        }
        Ok(())
    }

    pub fn set_bytes_at(&mut self, row: usize, col: usize, bytes: &[u8], wb: bool) -> io::Result<()> {
        for (i, b) in bytes.iter().enumerate() {
            let coli = col + i;
            if coli < LCD_WIDTH {
                if self.buffer[row][coli].val != *b {
                    self.buffer[row][coli].val = *b;
                    self.buffer[row][coli].dirty = true;
                }
            }
        }

        if wb {
            self.write_back()?;
        }
        Ok(())
    }
}

struct TextCanvas {
    bmpset:      bitmap_font::BitmapFont,
    upper_left:  [usize; 2],
    lower_right: [usize; 2],
}

impl TextCanvas {
    fn new(
        bmpset: &bitmap_font::BitmapFont,
        upper_left: [usize; 2],
        lower_right: [usize; 2],
    ) -> io::Result<TextCanvas> {
        if upper_left[0] >= lower_right[0] || upper_left[1] >= lower_right[1] {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "`upper_left` must be strictly to the left and above `lower_right`",
            ));
        }

        let canvas_height = (lower_right[1] - upper_left[1]) as u32;

        if canvas_height < bmpset.height() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "canvas height must be greater than or equal to font height",
            ));
        }

        Ok(TextCanvas { bmpset: *bmpset, upper_left, lower_right })
    }

    fn width(&self) -> usize {
        self.lower_right[0] - self.upper_left[0]
    }

    fn height(&self) -> usize {
        self.lower_right[1] - self.upper_left[1]
    }

    fn render_text(&self, dpy: &mut BufferedLcd, text: &str) -> io::Result<()> {
        let char_width = self.bmpset.width() as usize;
        let char_height = self.bmpset.height() as usize;
        let text_width = text.len() * char_width;

        if text_width > self.width() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Text is too long to fit in canvas width ({} vs {}, text: {})", text_width, self.width(), text),
            ));
        }

        let left_padding = (self.width() - text_width) / 2;
        let mut bits = Vec2d::new(self.height(), self.width());

        for (i_char, ch) in text.chars().enumerate() {
            for i_row in 0..char_height {
                for i_col_in_char in 0..char_width {
                    let i_col = left_padding + i_char * char_width + i_col_in_char;
                    bits.set(i_row, i_col, self.bmpset.pixel(ch, i_col_in_char as u32, i_row as u32));
                }
            }
        }

        dpy.set_bits_at(self.upper_left[1], self.upper_left[0], &bits, true)?;

        Ok(())
    }
}

pub struct Display {
    dev:           BufferedLcd,
    clock_canvas:  TextCanvas,
    top_canvas:    TextCanvas,
    bottom_canvas: TextCanvas,
}

impl Display {
    pub fn new() -> io::Result<Display> {
        let dev = BufferedLcd::new()?;

        let clock_canvas = TextCanvas::new(&bitmap_font::FONT_16x32, [0, 16], [128, 48])?;
        let top_canvas = TextCanvas::new(&bitmap_font::FONT_7x13, [0, 0], [128, 16])?;
        let bottom_canvas = TextCanvas::new(&bitmap_font::FONT_7x13, [0, 48], [128, 64])?;

        Ok(Display { dev, clock_canvas, top_canvas, bottom_canvas })
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.clock_canvas.render_text(&mut self.dev, "")?;
        self.top_canvas.render_text(&mut self.dev, "")?;
        self.bottom_canvas.render_text(&mut self.dev, "")
    }

    pub fn get_backlight(&mut self) -> bool {
        self.dev.get_backlight()
    }

    pub fn set_backlight(&mut self, on: bool) -> io::Result<()> {
        self.dev.set_backlight(on)
    }

    pub fn set_top_line(&mut self, line: &str) -> io::Result<()> {
        self.top_canvas.render_text(&mut self.dev, line)
    }

    pub fn set_bottom_line(&mut self, line: &str) -> io::Result<()> {
        self.bottom_canvas.render_text(&mut self.dev, line)
    }

    pub fn show_time(&mut self, now: &DateTime<Local>) -> io::Result<()> {
        let text = now.format("%H:%M").to_string();

        self.clock_canvas.render_text(&mut self.dev, &text)
    }
}
