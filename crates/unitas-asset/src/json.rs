use std::fmt;

const MAX_DEPTH: usize = 128;
const I64_LIMIT: f64 = 9_223_372_036_854_775_808.0;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsonError {
    UnexpectedEnd,
    UnexpectedChar(char),
    InvalidNumber,
    InvalidEscape,
    InvalidUtf8,
    TrailingData,
    DepthExceeded,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::UnexpectedEnd => f.write_str("unexpected end of input"),
            JsonError::UnexpectedChar(c) => write!(f, "unexpected character {c:?}"),
            JsonError::InvalidNumber => f.write_str("invalid number"),
            JsonError::InvalidEscape => f.write_str("invalid escape sequence"),
            JsonError::InvalidUtf8 => f.write_str("invalid UTF-8"),
            JsonError::TrailingData => f.write_str("trailing data after the top-level value"),
            JsonError::DepthExceeded => f.write_str("maximum nesting depth exceeded"),
        }
    }
}

impl std::error::Error for JsonError {}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => entries
                .iter()
                .find(|(entry_key, _)| entry_key == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(text) => Some(text.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(number) => Some(*number),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(number)
                if number.is_finite()
                    && number.fract() == 0.0
                    && *number >= -I64_LIMIT
                    && *number < I64_LIMIT =>
            {
                Some(*number as i64)
            }
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            JsonValue::Object(entries) => Some(entries.as_slice()),
            _ => None,
        }
    }

    pub fn to_json_string(&self) -> String {
        write(self)
    }
}

pub fn parse(text: &str) -> Result<JsonValue, JsonError> {
    Parser::new(text.as_bytes()).parse_document()
}

pub fn parse_bytes(bytes: &[u8]) -> Result<JsonValue, JsonError> {
    Parser::new(bytes).parse_document()
}

pub fn write(value: &JsonValue) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0);
    out.push('\n');
    out
}

fn write_value(out: &mut String, value: &JsonValue, depth: usize) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(number) => out.push_str(&number_to_string(*number)),
        JsonValue::String(text) => write_string(out, text),
        JsonValue::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, depth + 1);
                write_value(out, item, depth + 1);
            }
            out.push('\n');
            write_indent(out, depth);
            out.push(']');
        }
        JsonValue::Object(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (index, (key, entry_value)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, depth + 1);
                write_string(out, key);
                out.push_str(": ");
                write_value(out, entry_value, depth + 1);
            }
            out.push('\n');
            write_indent(out, depth);
            out.push('}');
        }
    }
}

fn write_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn number_to_string(number: f64) -> String {
    if !number.is_finite() {
        return "null".to_string();
    }
    format!("{number}")
}

fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Parser {
            bytes,
            pos: 0,
            depth: 0,
        }
    }

    fn parse_document(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.pos < self.bytes.len() {
            return Err(JsonError::TrailingData);
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        match self.peek() {
            None => Err(JsonError::UnexpectedEnd),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some(b'n') => self.parse_literal("null", JsonValue::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(_) => Err(self.unexpected_char()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.pos += 1;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(JsonError::DepthExceeded);
        }
        let mut entries: Vec<(String, JsonValue)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(match self.peek() {
                    None => JsonError::UnexpectedEnd,
                    Some(_) => self.unexpected_char(),
                });
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(match self.peek() {
                    None => JsonError::UnexpectedEnd,
                    Some(_) => self.unexpected_char(),
                });
            }
            self.pos += 1;
            self.skip_whitespace();
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                None => return Err(JsonError::UnexpectedEnd),
                Some(_) => return Err(self.unexpected_char()),
            }
        }
        self.depth -= 1;
        Ok(JsonValue::Object(entries))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.pos += 1;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(JsonError::DepthExceeded);
        }
        let mut items: Vec<JsonValue> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                None => return Err(JsonError::UnexpectedEnd),
                Some(_) => return Err(self.unexpected_char()),
            }
        }
        self.depth -= 1;
        Ok(JsonValue::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.pos += 1;
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            let byte = match self.peek() {
                None => return Err(JsonError::UnexpectedEnd),
                Some(byte) => byte,
            };
            match byte {
                b'"' => {
                    self.pos += 1;
                    break;
                }
                b'\\' => {
                    self.pos += 1;
                    self.parse_escape(&mut bytes)?;
                }
                0x00..=0x1f => return Err(JsonError::UnexpectedChar(byte as char)),
                _ => {
                    bytes.push(byte);
                    self.pos += 1;
                }
            }
        }
        String::from_utf8(bytes).map_err(|_| JsonError::InvalidUtf8)
    }

    fn parse_escape(&mut self, out: &mut Vec<u8>) -> Result<(), JsonError> {
        let byte = match self.peek() {
            None => return Err(JsonError::UnexpectedEnd),
            Some(byte) => byte,
        };
        self.pos += 1;
        match byte {
            b'"' => out.push(b'"'),
            b'\\' => out.push(b'\\'),
            b'/' => out.push(b'/'),
            b'b' => out.push(0x08),
            b'f' => out.push(0x0c),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'u' => {
                let code = self.parse_hex4()?;
                let character = self.decode_escape_code(code)?;
                let mut encoded = [0u8; 4];
                out.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            }
            _ => return Err(JsonError::InvalidEscape),
        }
        Ok(())
    }

    fn decode_escape_code(&mut self, code: u16) -> Result<char, JsonError> {
        let combined = if (0xd800..=0xdbff).contains(&code) {
            if self.peek() != Some(b'\\') {
                return Err(JsonError::InvalidEscape);
            }
            self.pos += 1;
            if self.peek() != Some(b'u') {
                return Err(JsonError::InvalidEscape);
            }
            self.pos += 1;
            let low = self.parse_hex4()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(JsonError::InvalidEscape);
            }
            0x10000 + ((u32::from(code) - 0xd800) << 10) + (u32::from(low) - 0xdc00)
        } else if (0xdc00..=0xdfff).contains(&code) {
            return Err(JsonError::InvalidEscape);
        } else {
            u32::from(code)
        };
        char::from_u32(combined).ok_or(JsonError::InvalidEscape)
    }

    fn parse_hex4(&mut self) -> Result<u16, JsonError> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let byte = match self.peek() {
                None => return Err(JsonError::UnexpectedEnd),
                Some(byte) => byte,
            };
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(JsonError::InvalidEscape),
            };
            value = value * 16 + u16::from(digit);
            self.pos += 1;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            None => return Err(JsonError::UnexpectedEnd),
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    return Err(JsonError::InvalidNumber);
                }
            }
            Some(c) if c.is_ascii_digit() => {
                while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            Some(_) => return Err(JsonError::InvalidNumber),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            match self.peek() {
                None => return Err(JsonError::UnexpectedEnd),
                Some(c) if c.is_ascii_digit() => {}
                Some(_) => return Err(JsonError::InvalidNumber),
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            match self.peek() {
                None => return Err(JsonError::UnexpectedEnd),
                Some(c) if c.is_ascii_digit() => {}
                Some(_) => return Err(JsonError::InvalidNumber),
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| JsonError::InvalidUtf8)?;
        let number: f64 = text.parse::<f64>().map_err(|_| JsonError::InvalidNumber)?;
        if !number.is_finite() {
            return Err(JsonError::InvalidNumber);
        }
        if let Some(c) = self.peek() {
            let is_delimiter = c == b',' || c == b']' || c == b'}' || c.is_ascii_whitespace();
            if !is_delimiter {
                return Err(JsonError::InvalidNumber);
            }
        }
        Ok(JsonValue::Number(number))
    }

    fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        let expected = literal.as_bytes();
        let mut index = 0;
        while index < expected.len()
            && self.pos + index < self.bytes.len()
            && self.bytes[self.pos + index] == expected[index]
        {
            index += 1;
        }
        if index < expected.len() {
            if self.pos + index >= self.bytes.len() {
                return Err(JsonError::UnexpectedEnd);
            }
            return Err(self.char_at(self.pos + index));
        }
        self.pos += expected.len();
        Ok(value)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn unexpected_char(&self) -> JsonError {
        self.char_at(self.pos)
    }

    fn char_at(&self, at: usize) -> JsonError {
        let first = match self.bytes.get(at) {
            None => return JsonError::UnexpectedEnd,
            Some(first) => *first,
        };
        let width = match first {
            0x00..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => return JsonError::InvalidUtf8,
        };
        let end = at + width;
        let slice = match self.bytes.get(at..end) {
            None => return JsonError::InvalidUtf8,
            Some(slice) => slice,
        };
        match std::str::from_utf8(slice) {
            Ok(text) => match text.chars().next() {
                Some(character) => JsonError::UnexpectedChar(character),
                None => JsonError::UnexpectedEnd,
            },
            Err(_) => JsonError::InvalidUtf8,
        }
    }
}
