
mod lcd;

use lcd::Lcd;
use std::time::Duration;
use std::thread::sleep;
use std::io::{self, Write};

const LCD_E: u64 = 21;
const LCD_RS: u64 = 20;
const LCD_D4: u64 = 6;
const LCD_D5: u64 = 13;
const LCD_D6: u64 = 19;
const LCD_D7: u64 = 26;

fn main() {
    let lcd = Lcd::new(LCD_E,LCD_RS,LCD_D4,LCD_D5,LCD_D6,LCD_D7);

    // Initialise display
    lcd.init();

    // Send some test
    lcd.print_string("Raspberry Pi", lcd::LCD_LINE_1);
    lcd.print_string("16x2 LCD Test", lcd::LCD_LINE_2);

    // 3 second delay
    sleep(Duration::new(3,0));
    // let mut i = 0;
    // while i < 256 {
    //     let line1 : Vec<u8> = (0..16).map(|x| (x+i) as u8).collect();
    //     let line2 : Vec<u8> = (16..32).map(|x| (x+i) as u8).collect();
    //     lcd_bytestr(&pins, &line1, LCD_LINE_1);
    //     lcd_bytestr(&pins, &line2, LCD_LINE_2);
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
        if line1.len() > lcd::LCD_WIDTH {
            line2 = &line1[lcd::LCD_WIDTH..];
            line1 = &line1[..lcd::LCD_WIDTH];
        }

        lcd.print_string(line1,lcd::LCD_LINE_1);
        lcd.print_string(line2,lcd::LCD_LINE_2);

        sleep(Duration::new(1,0));
    }

}
