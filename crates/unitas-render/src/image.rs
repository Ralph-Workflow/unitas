use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8 {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba8 {
    pub fn new(width: u32, height: u32) -> Self {
        let count = (width as usize) * (height as usize);
        Self {
            width,
            height,
            pixels: vec![0; count * 4],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.width && y < self.height {
            let offset = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
            [
                self.pixels[offset],
                self.pixels[offset + 1],
                self.pixels[offset + 2],
                self.pixels[offset + 3],
            ]
        } else {
            [0, 0, 0, 0]
        }
    }
}

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

    pub fn sample_bilinear(&self, u: f32, v: f32) -> [f32; 3] {
        let w = self.width as f32;
        let h = self.height as f32;
        if w < 1.0 || h < 1.0 {
            return [0.0; 3];
        }
        let x = (u * w - 0.5).clamp(0.0, w - 1.0);
        let y = (v * h - 0.5).clamp(0.0, h - 1.0);
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        let texel = |tx: u32, ty: u32| -> [f32; 3] {
            let p = self.pixels[(ty as usize) * (self.width as usize) + (tx as usize)];
            [
                f32::from(p[0]) / 255.0,
                f32::from(p[1]) / 255.0,
                f32::from(p[2]) / 255.0,
            ]
        };
        let c00 = texel(x0, y0);
        let c10 = texel(x1, y0);
        let c01 = texel(x0, y1);
        let c11 = texel(x1, y1);
        let mut out = [0.0f32; 3];
        for channel in 0..3 {
            let top = c00[channel] * (1.0 - fx) + c10[channel] * fx;
            let bottom = c01[channel] * (1.0 - fx) + c11[channel] * fx;
            out[channel] = top * (1.0 - fy) + bottom * fy;
        }
        out
    }

    pub fn to_rgba8(&self) -> Rgba8 {
        let mut rgba = Rgba8::new(self.width, self.height);
        for (index, pixel) in self.pixels.iter().enumerate() {
            let offset = index * 4;
            rgba.pixels[offset] = pixel[0];
            rgba.pixels[offset + 1] = pixel[1];
            rgba.pixels[offset + 2] = pixel[2];
            rgba.pixels[offset + 3] = 255;
        }
        rgba
    }

    pub fn from_rgba8(rgba: Rgba8) -> Image {
        let mut image = Image::new(rgba.width, rgba.height);
        for (index, pixel) in image.pixels.iter_mut().enumerate() {
            let offset = index * 4;
            pixel[0] = rgba.pixels[offset];
            pixel[1] = rgba.pixels[offset + 1];
            pixel[2] = rgba.pixels[offset + 2];
        }
        image
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
