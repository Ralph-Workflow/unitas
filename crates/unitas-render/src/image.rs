use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 3]>,
}

impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        let count = (width as usize) * (height as usize);
        Self {
            width,
            height,
            pixels: vec![[0, 0, 0]; count],
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 3] {
        if x < self.width && y < self.height {
            self.pixels[(y as usize) * (self.width as usize) + (x as usize)]
        } else {
            [0, 0, 0]
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, rgb: [u8; 3]) {
        if x < self.width && y < self.height {
            self.pixels[(y as usize) * (self.width as usize) + (x as usize)] = rgb;
        }
    }

    pub fn to_bmp_bytes(&self) -> Vec<u8> {
        let row_bytes = (self.width as usize) * 3;
        let row_stride = (row_bytes + 3) & !3;
        let padding = row_stride - row_bytes;
        let pixel_array_size = row_stride * (self.height as usize);
        let file_size = 54 + pixel_array_size;

        let mut out = Vec::with_capacity(file_size);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());

        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(self.width as i32).to_le_bytes());
        out.extend_from_slice(&(self.height as i32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(pixel_array_size as u32).to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());

        for y in (0..self.height).rev() {
            let row_start = (y as usize) * (self.width as usize);
            for x in 0..(self.width as usize) {
                let [r, g, b] = self.pixels[row_start + x];
                out.extend_from_slice(&[b, g, r]);
            }
            out.extend_from_slice(&[0u8; 3][..padding]);
        }

        out
    }

    pub fn write_bmp(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.to_bmp_bytes())
    }
}
