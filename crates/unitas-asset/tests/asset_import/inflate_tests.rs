use std::error::Error;

use unitas_asset::inflate::{adler32, inflate_raw, inflate_zlib, InflateError};

type TestResult = Result<(), Box<dyn Error>>;

struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    bit_pos: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            current: 0,
            bit_pos: 0,
        }
    }

    fn write_bit(&mut self, bit: u8) {
        self.current |= (bit & 1) << self.bit_pos;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.bit_pos = 0;
        }
    }

    fn write_bits_lsb(&mut self, value: u32, count: u8) {
        for shift in 0..count {
            self.write_bit(((value >> shift) & 1) as u8);
        }
    }

    fn write_code_msb(&mut self, code: u16, len: u8) {
        for shift in (0..len).rev() {
            self.write_bit(((code >> shift) & 1) as u8);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bit_pos != 0 {
            self.bytes.push(self.current);
        }
        self.bytes
    }
}

fn wrap_zlib(deflate: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x9C];
    out.extend_from_slice(deflate);
    out.extend_from_slice(&adler32(payload).to_be_bytes());
    out
}

fn fixed_literal_code(symbol: u16) -> (u16, u8) {
    if symbol <= 143 {
        (0x30 + symbol, 8)
    } else if symbol <= 255 {
        (0x190 + (symbol - 144), 9)
    } else if symbol <= 279 {
        (symbol - 256, 7)
    } else {
        (0xC0 + (symbol - 280), 8)
    }
}

#[test]
fn inflates_empty_and_single_stored_block() -> TestResult {
    let empty_block = [0x01u8, 0x00, 0x00, 0xFF, 0xFF];
    assert_eq!(inflate_raw(&empty_block)?, Vec::<u8>::new());

    let payload = b"Unitas DEFLATE stored block payload";
    let len = payload.len() as u16;
    let mut raw = vec![0x01];
    raw.extend_from_slice(&len.to_le_bytes());
    raw.extend_from_slice(&(!len).to_le_bytes());
    raw.extend_from_slice(payload);

    assert_eq!(inflate_raw(&raw)?, payload);
    let zlib = wrap_zlib(&raw, payload);
    assert_eq!(inflate_zlib(&zlib)?, payload);
    Ok(())
}

#[test]
fn inflates_multi_stored_blocks() -> TestResult {
    let part1 = b"first block ";
    let part2 = b"second block";
    let mut raw = Vec::new();

    let len1 = part1.len() as u16;
    raw.push(0x00);
    raw.extend_from_slice(&len1.to_le_bytes());
    raw.extend_from_slice(&(!len1).to_le_bytes());
    raw.extend_from_slice(part1);

    let len2 = part2.len() as u16;
    raw.push(0x01);
    raw.extend_from_slice(&len2.to_le_bytes());
    raw.extend_from_slice(&(!len2).to_le_bytes());
    raw.extend_from_slice(part2);

    assert_eq!(inflate_raw(&raw)?, b"first block second block");
    Ok(())
}

#[test]
fn inflates_fixed_huffman_literals_and_overlapping_back_reference() -> TestResult {
    let mut writer = BitWriter::new();
    writer.write_bit(1);
    writer.write_bits_lsb(1, 2);

    for &byte in b"abc" {
        let (code, len) = fixed_literal_code(u16::from(byte));
        writer.write_code_msb(code, len);
    }
    let (high_code, high_len) = fixed_literal_code(200);
    writer.write_code_msb(high_code, high_len);

    let (len_code, len_bits) = fixed_literal_code(261);
    writer.write_code_msb(len_code, len_bits);
    writer.write_code_msb(3, 5);

    let (eob_code, eob_len) = fixed_literal_code(256);
    writer.write_code_msb(eob_code, eob_len);

    let stream = writer.finish();
    let expected = vec![
        b'a', b'b', b'c', 200, b'a', b'b', b'c', 200, b'a', b'b', b'c',
    ];
    assert_eq!(inflate_raw(&stream)?, expected);
    assert_eq!(inflate_zlib(&wrap_zlib(&stream, &expected))?, expected);
    Ok(())
}

#[test]
fn inflates_fixed_huffman_single_byte_run_back_reference() -> TestResult {
    let mut writer = BitWriter::new();
    writer.write_bit(1);
    writer.write_bits_lsb(1, 2);

    let (lit_code, lit_len) = fixed_literal_code(u16::from(b'Z'));
    writer.write_code_msb(lit_code, lit_len);

    let (len_code, len_bits) = fixed_literal_code(265);
    writer.write_code_msb(len_code, len_bits);
    writer.write_bits_lsb(1, 1);
    writer.write_code_msb(0, 5);

    let (eob_code, eob_len) = fixed_literal_code(256);
    writer.write_code_msb(eob_code, eob_len);

    let stream = writer.finish();
    assert_eq!(inflate_raw(&stream)?, vec![b'Z'; 13]);
    Ok(())
}

#[test]
fn inflates_dynamic_huffman_block_with_rle_and_back_reference() -> TestResult {
    let raw: [u8; 30] = [
        0x78, 0x9C, 0x4B, 0x4C, 0x4A, 0x4E, 0x49, 0x4D, 0x4B, 0xCF, 0xC8, 0xCC, 0xCA, 0xCE, 0xC9,
        0xCD, 0xCB, 0x2F, 0x28, 0x2C, 0x2A, 0x2E, 0x29, 0x2D, 0x03, 0x00, 0x66, 0xDE, 0x09, 0x3E,
    ];
    assert_eq!(inflate_zlib(&raw)?, b"abcdefghijklmnopqrstuv");

    let dynamic_zlib: [u8; 44] = [
        0x78, 0x9C, 0xED, 0xC7, 0x37, 0x11, 0x00, 0x30, 0x0C, 0x04, 0x30, 0x4A, 0x29, 0x08, 0xDE,
        0xE6, 0xCF, 0x29, 0xC6, 0x91, 0xD3, 0xA0, 0x41, 0xA9, 0x5E, 0xA9, 0xDE, 0xE3, 0x8C, 0x1B,
        0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0xFF, 0xEE, 0x0F, 0x16, 0x4C, 0xDE, 0x2E,
    ];
    let mut expected = Vec::with_capacity(4096);
    for i in 0..1024 {
        expected.extend_from_slice(&[b'A', b'B', b'C', (i % 4) as u8 + b'0']);
    }
    let decoded = inflate_zlib(&dynamic_zlib)?;
    assert_eq!(decoded.len(), expected.len());
    assert_eq!(decoded, expected);
    Ok(())
}

#[test]
fn inflates_hand_built_dynamic_huffman_tree_with_code_length_rle() -> TestResult {
    let mut writer = BitWriter::new();
    writer.write_bit(1);
    writer.write_bits_lsb(2, 2);
    writer.write_bits_lsb(1, 5);
    writer.write_bits_lsb(1, 5);
    writer.write_bits_lsb(15, 4);

    let cl_order = [
        16usize, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut cl_lens = [0u8; 19];
    cl_lens[0] = 2;
    cl_lens[1] = 2;
    cl_lens[2] = 2;
    cl_lens[16] = 3;
    cl_lens[17] = 4;
    cl_lens[18] = 4;
    for &sym in &cl_order {
        writer.write_bits_lsb(u32::from(cl_lens[sym]), 3);
    }

    writer.write_code_msb(0b00, 2);
    writer.write_code_msb(0b1111, 4);
    writer.write_bits_lsb(53, 7);
    for _ in 0..3 {
        writer.write_code_msb(0b10, 2);
    }
    writer.write_code_msb(0b00, 2);
    writer.write_code_msb(0b1111, 4);
    writer.write_bits_lsb(127, 7);
    writer.write_code_msb(0b1111, 4);
    writer.write_bits_lsb(35, 7);
    writer.write_code_msb(0b110, 3);
    writer.write_bits_lsb(0, 2);
    writer.write_code_msb(0b10, 2);
    writer.write_code_msb(0b00, 2);
    writer.write_code_msb(0b01, 2);
    writer.write_code_msb(0b01, 2);

    for &byte in b"ABAAAB" {
        writer.write_code_msb(u16::from(byte != b'A'), 2);
    }
    writer.write_code_msb(0b11, 2);

    let stream = writer.finish();
    assert_eq!(inflate_raw(&stream)?, b"ABAAAB");
    Ok(())
}

#[test]
fn rejects_incomplete_or_oversubscribed_huffman_trees() -> TestResult {
    let mut writer = BitWriter::new();
    writer.write_bit(1);
    writer.write_bits_lsb(2, 2);
    writer.write_bits_lsb(0, 5);
    writer.write_bits_lsb(0, 5);
    writer.write_bits_lsb(0, 4);
    writer.write_bits_lsb(1, 3);
    writer.write_bits_lsb(1, 3);
    writer.write_bits_lsb(1, 3);
    writer.write_bits_lsb(0, 3);
    let stream = writer.finish();
    assert_eq!(inflate_raw(&stream), Err(InflateError::InvalidHuffmanTree));
    Ok(())
}

#[test]
fn rejects_adler32_mismatch() -> TestResult {
    let payload = b"checksum verification";
    let len = payload.len() as u16;
    let mut raw = vec![0x01];
    raw.extend_from_slice(&len.to_le_bytes());
    raw.extend_from_slice(&(!len).to_le_bytes());
    raw.extend_from_slice(payload);

    let mut zlib = wrap_zlib(&raw, payload);
    let last = zlib.len() - 1;
    zlib[last] ^= 0x01;

    match inflate_zlib(&zlib) {
        Err(InflateError::Adler32Mismatch { expected, actual }) => {
            assert_ne!(expected, actual);
        }
        other => return Err(format!("expected Adler32Mismatch, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn rejects_invalid_zlib_headers() -> TestResult {
    let bad_cm = [0x79u8, 0x94, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01];
    assert_eq!(inflate_zlib(&bad_cm), Err(InflateError::InvalidZlibHeader));

    let bad_cinfo = [0x88u8, 0x10, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01];
    assert_eq!(
        inflate_zlib(&bad_cinfo),
        Err(InflateError::InvalidZlibHeader)
    );

    let bad_fcheck = [0x78u8, 0x9D, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01];
    assert_eq!(
        inflate_zlib(&bad_fcheck),
        Err(InflateError::InvalidZlibHeader)
    );

    let fdict_set = [0x78u8, 0xBB, 0x00, 0x00, 0x00, 0x01, 0x03, 0x00];
    assert_eq!(
        inflate_zlib(&fdict_set),
        Err(InflateError::PresetDictionaryUnsupported)
    );
    Ok(())
}

#[test]
fn rejects_invalid_stored_block_nlen() -> TestResult {
    let bad_stored = [0x01u8, 0x02, 0x00, 0x00, 0x00, b'a', b'b'];
    assert_eq!(
        inflate_raw(&bad_stored),
        Err(InflateError::InvalidStoredBlockLength)
    );
    Ok(())
}

#[test]
fn rejects_reserved_block_type_three() -> TestResult {
    let reserved = [0b0000_0111u8];
    assert_eq!(
        inflate_raw(&reserved),
        Err(InflateError::InvalidBlockType(3))
    );
    Ok(())
}

#[test]
fn rejects_truncated_streams() -> TestResult {
    assert_eq!(
        inflate_zlib(&[0x78, 0x9C, 0x01]),
        Err(InflateError::Truncated)
    );
    assert_eq!(inflate_raw(&[]), Err(InflateError::Truncated));
    assert_eq!(
        inflate_raw(&[0x01, 0x04, 0x00, 0xFB, 0xFF, b'a', b'b']),
        Err(InflateError::Truncated)
    );

    let raw_stored = [0x01u8, 0x00, 0x00, 0xFF, 0xFF];
    let mut zlib_short_adler = vec![0x78, 0x9C];
    zlib_short_adler.extend_from_slice(&raw_stored);
    zlib_short_adler.extend_from_slice(&[0x00, 0x00]);
    assert_eq!(
        inflate_zlib(&zlib_short_adler),
        Err(InflateError::Truncated)
    );
    Ok(())
}

#[test]
fn rejects_distance_exceeding_output_window() -> TestResult {
    let mut writer = BitWriter::new();
    writer.write_bit(1);
    writer.write_bits_lsb(1, 2);

    let (len_code, len_bits) = fixed_literal_code(257);
    writer.write_code_msb(len_code, len_bits);
    writer.write_code_msb(0, 5);

    let stream = writer.finish();
    assert_eq!(
        inflate_raw(&stream),
        Err(InflateError::DistanceOutOfBounds {
            distance: 1,
            available: 0,
        })
    );
    Ok(())
}
