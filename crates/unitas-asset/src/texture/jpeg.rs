use jpeg_decoder::{Decoder, Error as JpegError, PixelFormat};

pub use crate::texture::import::{DecodedTexture, TextureImportError};
use crate::texture::import::TextureFormat;

pub const SIGNATURE: [u8; 3] = [0xFF, 0xD8, 0xFF];

pub fn has_jpeg_signature(bytes: &[u8]) -> bool {
    bytes.len() >= SIGNATURE.len() && bytes[..SIGNATURE.len()] == SIGNATURE
}

pub fn decode_jpeg(bytes: &[u8]) -> Result<DecodedTexture, TextureImportError> {
    if !has_jpeg_signature(bytes) {
        return Err(TextureImportError::InvalidSignature);
    }
    let mut decoder = Decoder::new(bytes);
    let samples = decoder.decode().map_err(map_jpeg_error)?;
    let info = match decoder.info() {
        Some(info) => info,
        None => {
            return Err(TextureImportError::CorruptData(
                "missing JPEG image info".to_string(),
            ))
        }
    };
    let width = u32::from(info.width);
    let height = u32::from(info.height);
    if width == 0 || height == 0 {
        return Err(TextureImportError::CorruptData("zero image size".to_string()));
    }
    let pixel_count = (width as usize).saturating_mul(height as usize);
    let pixels = to_rgba8(info.pixel_format, &samples, pixel_count)?;
    Ok(DecodedTexture {
        width,
        height,
        format: TextureFormat::Rgba8,
        pixels,
    })
}

fn map_jpeg_error(error: JpegError) -> TextureImportError {
    match error {
        JpegError::Io(io_err) => {
            if io_err.kind() == std::io::ErrorKind::UnexpectedEof {
                TextureImportError::Truncated
            } else {
                TextureImportError::CorruptData(io_err.to_string())
            }
        }
        other => TextureImportError::CorruptData(other.to_string()),
    }
}

fn to_rgba8(
    pixel_format: PixelFormat,
    samples: &[u8],
    pixel_count: usize,
) -> Result<Vec<u8>, TextureImportError> {
    let mut rgba = Vec::with_capacity(pixel_count.saturating_mul(4));
    match pixel_format {
        PixelFormat::L8 => {
            if samples.len() < pixel_count {
                return Err(TextureImportError::Truncated);
            }
            for &luma in &samples[..pixel_count] {
                rgba.extend_from_slice(&[luma, luma, luma, 255]);
            }
        }
        PixelFormat::L16 => {
            let needed = pixel_count.saturating_mul(2);
            if samples.len() < needed {
                return Err(TextureImportError::Truncated);
            }
            for chunk in samples[..needed].chunks_exact(2) {
                let value = u16::from_ne_bytes([chunk[0], chunk[1]]);
                let luma = (value >> 8) as u8;
                rgba.extend_from_slice(&[luma, luma, luma, 255]);
            }
        }
        PixelFormat::RGB24 => {
            let needed = pixel_count.saturating_mul(3);
            if samples.len() < needed {
                return Err(TextureImportError::Truncated);
            }
            for rgb in samples[..needed].chunks_exact(3) {
                rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        PixelFormat::CMYK32 => {
            let needed = pixel_count.saturating_mul(4);
            if samples.len() < needed {
                return Err(TextureImportError::Truncated);
            }
            for cmyk in samples[..needed].chunks_exact(4) {
                let c = u16::from(cmyk[0]);
                let m = u16::from(cmyk[1]);
                let y = u16::from(cmyk[2]);
                let k = u16::from(cmyk[3]);
                let inv_c = 255u16.saturating_sub(c);
                let inv_m = 255u16.saturating_sub(m);
                let inv_y = 255u16.saturating_sub(y);
                let inv_k = 255u16.saturating_sub(k);
                let r = ((inv_c * inv_k + 127) / 255) as u8;
                let g = ((inv_m * inv_k + 127) / 255) as u8;
                let b = ((inv_y * inv_k + 127) / 255) as u8;
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    Ok(rgba)
}
