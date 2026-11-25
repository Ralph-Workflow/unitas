use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InflateError {
    Truncated,
    InvalidZlibHeader,
    PresetDictionaryUnsupported,
    InvalidBlockType(u8),
    InvalidStoredBlockLength,
    InvalidHuffmanTree,
    InvalidHuffmanCode,
    InvalidLengthSymbol(u16),
    InvalidDistanceSymbol(u16),
    DistanceOutOfBounds { distance: usize, available: usize },
    Adler32Mismatch { expected: u32, actual: u32 },
}

impl fmt::Display for InflateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("deflate stream is truncated"),
            Self::InvalidZlibHeader => formatter.write_str("invalid zlib header"),
            Self::PresetDictionaryUnsupported => {
                formatter.write_str("zlib preset dictionary is not supported")
            }
            Self::InvalidBlockType(kind) => {
                write!(formatter, "reserved deflate block type {kind}")
            }
            Self::InvalidStoredBlockLength => {
                formatter.write_str("stored block length does not match one's complement")
            }
            Self::InvalidHuffmanTree => formatter.write_str("invalid Huffman code length tree"),
            Self::InvalidHuffmanCode => formatter.write_str("unmapped Huffman code in stream"),
            Self::InvalidLengthSymbol(symbol) => {
                write!(formatter, "invalid length symbol {symbol}")
            }
            Self::InvalidDistanceSymbol(symbol) => {
                write!(formatter, "invalid distance symbol {symbol}")
            }
            Self::DistanceOutOfBounds {
                distance,
                available,
            } => {
                write!(
                    formatter,
                    "back-reference distance {distance} exceeds available output {available}"
                )
            }
            Self::Adler32Mismatch { expected, actual } => {
                write!(
                    formatter,
                    "adler32 mismatch: expected {expected:#010x}, got {actual:#010x}"
                )
            }
        }
    }
}

impl std::error::Error for InflateError {}

const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];

const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

const MAX_BITS: usize = 15;

pub fn adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for chunk in data.chunks(5552) {
        for &byte in chunk {
            s1 += u32::from(byte);
            s2 += s1;
        }
        s1 %= MOD_ADLER;
        s2 %= MOD_ADLER;
    }
    (s2 << 16) | s1
}

pub fn inflate_zlib(input: &[u8]) -> Result<Vec<u8>, InflateError> {
    if input.len() < 6 {
        return Err(InflateError::Truncated);
    }
    let cmf = input[0];
    let flg = input[1];
    let cm = cmf & 0x0F;
    let cinfo = cmf >> 4;
    if cm != 8 || cinfo > 7 {
        return Err(InflateError::InvalidZlibHeader);
    }
    let header_word = (u16::from(cmf) << 8) | u16::from(flg);
    if header_word % 31 != 0 {
        return Err(InflateError::InvalidZlibHeader);
    }
    if (flg & 0x20) != 0 {
        return Err(InflateError::PresetDictionaryUnsupported);
    }
    let mut reader = BitReader::new(&input[2..]);
    let output = inflate_stream(&mut reader)?;
    reader.align_to_byte();
    let checksum_bytes = reader.read_bytes(4)?;
    let expected = u32::from_be_bytes([
        checksum_bytes[0],
        checksum_bytes[1],
        checksum_bytes[2],
        checksum_bytes[3],
    ]);
    let actual = adler32(&output);
    if expected != actual {
        return Err(InflateError::Adler32Mismatch { expected, actual });
    }
    Ok(output)
}

pub fn inflate_raw(input: &[u8]) -> Result<Vec<u8>, InflateError> {
    let mut reader = BitReader::new(input);
    inflate_stream(&mut reader)
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bit(&mut self) -> Result<u8, InflateError> {
        if self.byte_pos >= self.data.len() {
            return Err(InflateError::Truncated);
        }
        let bit = (self.data[self.byte_pos] >> self.bit_pos) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Ok(bit)
    }

    fn read_bits(&mut self, count: u8) -> Result<u16, InflateError> {
        let mut value = 0u16;
        for shift in 0..count {
            let bit = u16::from(self.read_bit()?);
            value |= bit << shift;
        }
        Ok(value)
    }

    fn align_to_byte(&mut self) {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], InflateError> {
        self.align_to_byte();
        let end = match self.byte_pos.checked_add(len) {
            Some(end) if end <= self.data.len() => end,
            _ => return Err(InflateError::Truncated),
        };
        let slice = &self.data[self.byte_pos..end];
        self.byte_pos = end;
        Ok(slice)
    }
}

struct HuffmanTree {
    counts: [u16; MAX_BITS + 1],
    symbols: Vec<u16>,
    max_len: usize,
}

impl HuffmanTree {
    fn from_lengths(lengths: &[u8]) -> Result<Self, InflateError> {
        let mut counts = [0u16; MAX_BITS + 1];
        let mut max_len = 0usize;
        for &length in lengths {
            let len = usize::from(length);
            if len > MAX_BITS {
                return Err(InflateError::InvalidHuffmanTree);
            }
            if len > 0 {
                counts[len] += 1;
                if len > max_len {
                    max_len = len;
                }
            }
        }
        if max_len == 0 {
            return Ok(Self {
                counts,
                symbols: Vec::new(),
                max_len: 0,
            });
        }
        let mut available = 1i32;
        for &count in &counts[1..=MAX_BITS] {
            available <<= 1;
            available -= i32::from(count);
            if available < 0 {
                return Err(InflateError::InvalidHuffmanTree);
            }
        }
        let mut offsets = [0u16; MAX_BITS + 1];
        let mut total = 0u16;
        for bits in 1..=max_len {
            offsets[bits] = total;
            total += counts[bits];
        }
        let mut symbols = vec![0u16; usize::from(total)];
        for (symbol, &length) in lengths.iter().enumerate() {
            let len = usize::from(length);
            if len > 0 {
                let slot = usize::from(offsets[len]);
                symbols[slot] = symbol as u16;
                offsets[len] += 1;
            }
        }
        Ok(Self {
            counts,
            symbols,
            max_len,
        })
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, InflateError> {
        if self.max_len == 0 {
            return Err(InflateError::InvalidHuffmanCode);
        }
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0i32;
        for len in 1..=self.max_len {
            code |= i32::from(reader.read_bit()?);
            let count = i32::from(self.counts[len]);
            if code - count < first {
                let symbol_index = (index + (code - first)) as usize;
                return self
                    .symbols
                    .get(symbol_index)
                    .copied()
                    .ok_or(InflateError::InvalidHuffmanCode);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(InflateError::InvalidHuffmanCode)
    }
}

fn inflate_stream(reader: &mut BitReader<'_>) -> Result<Vec<u8>, InflateError> {
    let mut output = Vec::new();
    loop {
        let is_final = reader.read_bit()? == 1;
        let block_type = reader.read_bits(2)? as u8;
        match block_type {
            0 => inflate_stored_block(reader, &mut output)?,
            1 => {
                let (lit_len_tree, dist_tree) = fixed_trees()?;
                inflate_compressed_block(reader, &lit_len_tree, &dist_tree, &mut output)?;
            }
            2 => {
                let (lit_len_tree, dist_tree) = read_dynamic_trees(reader)?;
                inflate_compressed_block(reader, &lit_len_tree, &dist_tree, &mut output)?;
            }
            other => return Err(InflateError::InvalidBlockType(other)),
        }
        if is_final {
            break;
        }
    }
    Ok(output)
}

fn inflate_stored_block(
    reader: &mut BitReader<'_>,
    output: &mut Vec<u8>,
) -> Result<(), InflateError> {
    let header = reader.read_bytes(4)?;
    let len = u16::from_le_bytes([header[0], header[1]]);
    let nlen = u16::from_le_bytes([header[2], header[3]]);
    if len != !nlen {
        return Err(InflateError::InvalidStoredBlockLength);
    }
    let payload = reader.read_bytes(usize::from(len))?;
    output.extend_from_slice(payload);
    Ok(())
}

fn fixed_trees() -> Result<(HuffmanTree, HuffmanTree), InflateError> {
    let mut lit_lengths = [0u8; 288];
    for length in &mut lit_lengths[0..=143] {
        *length = 8;
    }
    for length in &mut lit_lengths[144..=255] {
        *length = 9;
    }
    for length in &mut lit_lengths[256..=279] {
        *length = 7;
    }
    for length in &mut lit_lengths[280..=287] {
        *length = 8;
    }
    let dist_lengths = [5u8; 32];
    Ok((
        HuffmanTree::from_lengths(&lit_lengths)?,
        HuffmanTree::from_lengths(&dist_lengths)?,
    ))
}

fn read_dynamic_trees(
    reader: &mut BitReader<'_>,
) -> Result<(HuffmanTree, HuffmanTree), InflateError> {
    let hlit = usize::from(reader.read_bits(5)?) + 257;
    let hdist = usize::from(reader.read_bits(5)?) + 1;
    let hclen = usize::from(reader.read_bits(4)?) + 4;
    if hlit > 286 || hdist > 30 {
        return Err(InflateError::InvalidHuffmanTree);
    }
    let mut code_length_lengths = [0u8; 19];
    for i in 0..hclen {
        code_length_lengths[CODE_LENGTH_ORDER[i]] = reader.read_bits(3)? as u8;
    }
    let code_length_tree = HuffmanTree::from_lengths(&code_length_lengths)?;
    let total_codes = hlit + hdist;
    let mut lengths = Vec::with_capacity(total_codes);
    while lengths.len() < total_codes {
        let symbol = code_length_tree.decode(reader)?;
        match symbol {
            0..=15 => lengths.push(symbol as u8),
            16 => {
                let prev = *lengths.last().ok_or(InflateError::InvalidHuffmanTree)?;
                let repeat = usize::from(reader.read_bits(2)?) + 3;
                if lengths.len() + repeat > total_codes {
                    return Err(InflateError::InvalidHuffmanTree);
                }
                lengths.resize(lengths.len() + repeat, prev);
            }
            17 => {
                let repeat = usize::from(reader.read_bits(3)?) + 3;
                if lengths.len() + repeat > total_codes {
                    return Err(InflateError::InvalidHuffmanTree);
                }
                lengths.resize(lengths.len() + repeat, 0);
            }
            18 => {
                let repeat = usize::from(reader.read_bits(7)?) + 11;
                if lengths.len() + repeat > total_codes {
                    return Err(InflateError::InvalidHuffmanTree);
                }
                lengths.resize(lengths.len() + repeat, 0);
            }
            _ => return Err(InflateError::InvalidHuffmanTree),
        }
    }
    if lengths[256] == 0 {
        return Err(InflateError::InvalidHuffmanTree);
    }
    let lit_len_tree = HuffmanTree::from_lengths(&lengths[..hlit])?;
    let dist_tree = HuffmanTree::from_lengths(&lengths[hlit..])?;
    Ok((lit_len_tree, dist_tree))
}

fn inflate_compressed_block(
    reader: &mut BitReader<'_>,
    lit_len_tree: &HuffmanTree,
    dist_tree: &HuffmanTree,
    output: &mut Vec<u8>,
) -> Result<(), InflateError> {
    loop {
        let symbol = lit_len_tree.decode(reader)?;
        match symbol.cmp(&256) {
            std::cmp::Ordering::Less => output.push(symbol as u8),
            std::cmp::Ordering::Equal => break,
            std::cmp::Ordering::Greater => {
                let length_index = usize::from(symbol - 257);
                if length_index >= LENGTH_BASE.len() {
                    return Err(InflateError::InvalidLengthSymbol(symbol));
                }
                let extra_bits = LENGTH_EXTRA[length_index];
                let extra = if extra_bits > 0 {
                    reader.read_bits(extra_bits)?
                } else {
                    0
                };
                let length = usize::from(LENGTH_BASE[length_index] + extra);

                let dist_symbol = dist_tree.decode(reader)?;
                let dist_index = usize::from(dist_symbol);
                if dist_index >= DISTANCE_BASE.len() {
                    return Err(InflateError::InvalidDistanceSymbol(dist_symbol));
                }
                let dist_extra_bits = DISTANCE_EXTRA[dist_index];
                let dist_extra = if dist_extra_bits > 0 {
                    reader.read_bits(dist_extra_bits)?
                } else {
                    0
                };
                let distance = usize::from(DISTANCE_BASE[dist_index] + dist_extra);
                if distance == 0 || distance > output.len() {
                    return Err(InflateError::DistanceOutOfBounds {
                        distance,
                        available: output.len(),
                    });
                }
                for _ in 0..length {
                    let byte = output[output.len() - distance];
                    output.push(byte);
                }
            }
        }
    }
    Ok(())
}
