
mod lcd;

use lcd::Lcd;
use std::time::Duration;
use std::thread::sleep;
use std::io::{self, Write};

const I2C_PATH: &str = "/dev/i2c-1";
const LCD_ADDR: u16 = 0x27;

fn main() {
    let mut lcd = Lcd::new(I2C_PATH, LCD_ADDR);

    // Initialise display
    lcd.init();

    // Send some test
    lcd.print_string("Raspberry Pi", lcd::LINE_1);
    lcd.print_string("16x2 LCD Test", lcd::LINE_2);

    // 3 second delay
    sleep(Duration::new(3,0));
    // let mut i = 0;
    // while i < 256 {
    //     let line1 : Vec<u8> = (0..16).map(|x| (x+i) as u8).collect();
    //     let line2 : Vec<u8> = (16..32).map(|x| (x+i) as u8).collect();
    //     lcd.send_bytestr(&line1, lcd::LINE_1);
    //     lcd.send_bytestr(&line2, lcd::LINE_2);
    //     i += 32;

    //     println!("currently displaying:");
    //     for d in line1 {
    //         print!("{} ",d);
    //     }
    //     println!();
    //     for d in line2 {
    //         print!("{} ",d);
    //     }
    //     println!();
    //     sleep(Duration::new(1,0));
    // }

    loop {
        let mut input = String::new();
        print!("write something to display:");
        let _=io::stdout().flush();
        io::stdin().read_line(&mut input).expect("Failed reading input");
        let mut line1 = input.trim();

        if line1.len() == 0 {
            break;
        }

        let mut line2= "";
        if line1.len() > lcd::WIDTH {
            line2 = &line1[lcd::WIDTH..];
            line1 = &line1[..lcd::WIDTH];
        }

        lcd.print_string(line1,lcd::LINE_1);
        lcd.print_string(line2,lcd::LINE_2);

        sleep(Duration::new(1,0));
    }

    lcd.print_string("Bye!",lcd::LINE_1);
    lcd.print_string("",lcd::LINE_2);
}
