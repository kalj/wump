extern crate rusttype;

use std::collections::{HashMap, HashSet};
use std::cmp::{min,max};
use self::rusttype::{point, Font, Scale, PositionedGlyph};

pub struct FontBitmapSet {
    bitmaps: HashMap<char, Vec<bool>>,
    width: u32,
    height: u32
}

impl FontBitmapSet {

    pub fn new(font_data: &[u8], font_size: u32 ) -> FontBitmapSet {
        let chars: HashSet<_> = (32..255).filter_map(|u| {
            let c = char::from(u);
            if c.is_control() {
                None
            } else {
                Some(c)
            }
        }).collect();

        Self::new_with_charset(font_data, font_size, &chars)
    }

    pub fn new_with_charset(font_data: &[u8], font_size: u32, charset: &HashSet<char>) -> FontBitmapSet {
        // This only succeeds if collection consists of one font
        let font = Font::try_from_bytes(font_data).expect("Error constructing Font");

        let mut glyphs: Vec<(char, PositionedGlyph)> = Vec::new();
        let mut glyph_hoffset: u32 = 0;
        let mut glyph_voffset: u32 = 0;
        let mut glyph_width: u32 = 0;
        let mut glyph_height: u32 = 0;
        let mut _actual_font_size: u32 = 0;

        let mut i: i32 = -1;
        loop {

            let scale = Scale::uniform((font_size as i32 + i) as f32);

            let v_metrics = font.v_metrics(scale);
            let offset = point(0.0, v_metrics.ascent);

            let trial_glyphs: Vec<_> = charset.iter().map(|&c| {
                let glyph = font.glyph(c).scaled(scale).positioned(offset);
                (c,glyph)
            }).collect();

            // work out the glyph width and height
            let (trial_glyph_hoffset, trial_glyph_voffset, trial_glyph_width,trial_glyph_height) = {
                let mut min_x = 2000;
                let mut max_x = 0;
                let mut min_y = 2000;
                let mut max_y = 0;
                for (_,g) in &trial_glyphs {
                    if let Some(bb) = g.pixel_bounding_box() {
                        min_x = min(min_x, bb.min.x);
                        max_x = max(max_x, bb.max.x);
                        min_y = min(min_y, bb.min.y);
                        max_y = max(max_y, bb.max.y);
                    }
                }
                (min_x as u32, min_y as u32, (1+max_x - min_x) as u32, (1+max_y - min_y) as u32)
            };

            if trial_glyph_height > font_size {
                break;
            } else {
                _actual_font_size = (font_size as i32 + i) as u32;
                glyphs = trial_glyphs;
                glyph_hoffset = trial_glyph_hoffset;
                glyph_voffset = trial_glyph_voffset;
                glyph_width = trial_glyph_width;
                glyph_height = trial_glyph_height;
                i += 1;
            }
        }

        let mut bitmaps: HashMap<char, Vec<bool>> = HashMap::new();

        for (ucode, glyph) in glyphs {

            let c = char::from(ucode);
            let mut bitmap = vec![false; (glyph_width*(glyph_height)) as usize];

            if let Some(bb) = glyph.pixel_bounding_box() {

                glyph.draw(|x,y,v| {
                    let col = x as i32 + bb.min.x - glyph_hoffset as i32;
                    let row = y as i32 + bb.min.y - glyph_voffset as i32;

                    if row < 0 || row >= (glyph_height as i32) || col < 0 || col >= (glyph_width as i32) {
                        panic!("Writing pixel data at ({}, {}) which is outside of bitmap with size ({}, {}). x: {}, y: {}, bb: {:?}", row, col, glyph_height, glyph_width, x, y, bb);
                    }

                    // bitmap[((col as u32) + (row as u32*glyph_width)) as usize] = v > 0.0;
                    bitmap[((col as u32) + (row as u32*glyph_width)) as usize] = v > 0.2;
                });
            }
            bitmaps.insert(c, bitmap );
        }

        FontBitmapSet { bitmaps, width: glyph_width, height: glyph_height }
    }

    pub fn glyph_height(&self) -> u32 {
        self.height
    }

    pub fn glyph_width(&self) -> u32 {
        self.width
    }

    pub fn get(&self, c: char, row: u32, col: u32) -> bool {
        if let Some(bmp) = self.bitmaps.get(&c) {
            bmp[(row*self.width + col) as usize]
        } else {
            panic!("No bitmap found for character {} with code {}", c, c as u8);
        }
    }
}
