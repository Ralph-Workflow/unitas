use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Scalar(String),
    Mapping(Vec<(String, Node)>),
    Sequence(Vec<Node>),
}

pub type YamlNode = Node;

impl Node {
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Node::Scalar(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_scalar()
    }

    pub fn as_i64(&self) -> Option<i64> {
        let text = self.as_str()?.trim();
        text.parse::<i64>().ok()
    }

    pub fn as_f64(&self) -> Option<f64> {
        let text = self.as_str()?.trim();
        text.parse::<f64>().ok()
    }

    pub fn as_mapping(&self) -> Option<&[(String, Node)]> {
        match self {
            Node::Mapping(entries) => Some(entries.as_slice()),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&[Node]> {
        match self {
            Node::Sequence(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Node> {
        match self {
            Node::Mapping(entries) => {
                entries.iter().find(|(entry_key, _)| entry_key == key).map(|(_, value)| value)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub class_id: u32,
    pub file_id: i64,
    pub stripped: bool,
    pub root: Node,
}

impl Document {
    pub fn file_id_u64(&self) -> Option<u64> {
        if self.file_id >= 0 {
            Some(self.file_id as u64)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub yaml_version: String,
    pub tag_handle: Option<String>,
    pub tag_prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub header: Option<Header>,
    pub documents: Vec<Document>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YamlError {
    TabIndentation { line: usize },
    InvalidDocumentHeader { line: usize, text: String },
    InvalidClassId { line: usize, text: String },
    InvalidFileId { line: usize, text: String },
    UnexpectedContent { line: usize, text: String },
    UnexpectedIndent { line: usize, expected: usize },
    UnterminatedString { line: usize },
    UnterminatedFlow { line: usize },
    InvalidEscape { line: usize, text: String },
    Io { path: PathBuf, message: String },
}

impl fmt::Display for YamlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YamlError::TabIndentation { line } => {
                write!(formatter, "line {}: tab character in indentation", line)
            }
            YamlError::InvalidDocumentHeader { line, text } => {
                write!(formatter, "line {}: invalid document header {}", line, text)
            }
            YamlError::InvalidClassId { line, text } => {
                write!(formatter, "line {}: invalid class id {}", line, text)
            }
            YamlError::InvalidFileId { line, text } => {
                write!(formatter, "line {}: invalid file id {}", line, text)
            }
            YamlError::UnexpectedContent { line, text } => {
                write!(formatter, "line {}: unexpected content {}", line, text)
            }
            YamlError::UnexpectedIndent { line, expected } => {
                write!(formatter, "line {}: unexpected indentation, expected {}", line, expected)
            }
            YamlError::UnterminatedString { line } => {
                write!(formatter, "line {}: unterminated quoted scalar", line)
            }
            YamlError::UnterminatedFlow { line } => {
                write!(formatter, "line {}: unterminated flow collection", line)
            }
            YamlError::InvalidEscape { line, text } => {
                write!(formatter, "line {}: invalid escape {}", line, text)
            }
            YamlError::Io { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
        }
    }
}

impl std::error::Error for YamlError {}

pub type AssetError = YamlError;

pub fn parse(text: &str) -> Result<Stream, YamlError> {
    let lines = split_lines(text)?;
    let mut parser = Parser { lines, position: 0 };
    let header = parser.parse_header()?;
    let mut documents = Vec::new();
    while parser.current().is_some() {
        documents.push(parser.parse_document()?);
    }
    Ok(Stream { header, documents })
}

pub fn parse_file(path: &Path) -> Result<Stream, YamlError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            return Err(YamlError::Io { path: path.to_path_buf(), message: error.to_string() })
        }
    };
    parse(&text)
}

#[derive(Debug, Clone, Copy)]
struct Line<'a> {
    indent: usize,
    text: &'a str,
    number: usize,
}

fn split_lines(text: &str) -> Result<Vec<Line<'_>>, YamlError> {
    let mut lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let trimmed = raw.trim_start_matches([' ', '\t']);
        let whitespace = &raw[..raw.len() - trimmed.len()];
        if whitespace.contains('\t') {
            return Err(YamlError::TabIndentation { line: number });
        }
        let content = &raw[whitespace.len()..];
        if content.trim().is_empty() {
            continue;
        }
        emit(content, whitespace.len(), number, &mut lines);
    }
    Ok(lines)
}

fn emit<'a>(text: &'a str, column: usize, number: usize, lines: &mut Vec<Line<'a>>) {
    if text == "-" || text.starts_with("- ") {
        lines.push(Line { indent: column, text: "-", number });
        let rest = &text[1..];
        let content = rest.trim_start_matches(' ');
        let content_column = column + 1 + rest.len() - content.len();
        if !content.is_empty() {
            emit(content, content_column, number, lines);
        }
    } else {
        lines.push(Line { indent: column, text, number });
    }
}

struct Parser<'a> {
    lines: Vec<Line<'a>>,
    position: usize,
}

impl<'a> Parser<'a> {
    fn current(&self) -> Option<Line<'a>> {
        self.lines.get(self.position).copied()
    }

    fn parse_header(&mut self) -> Result<Option<Header>, YamlError> {
        let mut yaml_version: Option<String> = None;
        let mut tag_handle: Option<String> = None;
        let mut tag_prefix: Option<String> = None;
        while let Some(line) = self.current() {
            if !line.text.starts_with('%') {
                break;
            }
            let directive = &line.text[1..];
            if let Some(value) = directive.strip_prefix("YAML") {
                yaml_version = Some(value.trim().to_string());
            } else if let Some(value) = directive.strip_prefix("TAG") {
                let mut parts = value.trim_start().splitn(2, char::is_whitespace);
                tag_handle = Some(parts.next().unwrap_or_default().trim().to_string());
                tag_prefix = Some(parts.next().unwrap_or_default().trim().to_string());
            }
            self.position += 1;
        }
        match yaml_version {
            Some(version) => Ok(Some(Header { yaml_version: version, tag_handle, tag_prefix })),
            None => Ok(None),
        }
    }

    fn parse_document(&mut self) -> Result<Document, YamlError> {
        let line = match self.current() {
            Some(line) => line,
            None => {
                return Err(YamlError::UnexpectedContent {
                    line: 0,
                    text: String::from("expected a document"),
                })
            }
        };
        let rest = match line.text.strip_prefix("---") {
            Some(rest) => rest.trim_start(),
            None => {
                return Err(YamlError::UnexpectedContent {
                    line: line.number,
                    text: line.text.to_string(),
                })
            }
        };
        let tag = match rest.strip_prefix("!u!") {
            Some(tag) => tag,
            None => {
                return Err(YamlError::InvalidDocumentHeader {
                    line: line.number,
                    text: line.text.to_string(),
                })
            }
        };
        let digit_end = tag.find(|character: char| !character.is_ascii_digit()).unwrap_or(tag.len());
        if digit_end == 0 {
            return Err(YamlError::InvalidClassId { line: line.number, text: rest.to_string() });
        }
        let class_id = match tag[..digit_end].parse::<u32>() {
            Ok(class_id) => class_id,
            Err(_) => {
                return Err(YamlError::InvalidClassId { line: line.number, text: rest.to_string() })
            }
        };
        let mut cursor = tag[digit_end..].trim();
        let mut file_id = 0i64;
        if let Some(anchor) = cursor.strip_prefix('&') {
            let id_end = anchor
                .find(|character: char| !(character.is_ascii_digit() || character == '-'))
                .unwrap_or(anchor.len());
            file_id = match anchor[..id_end].parse::<i64>() {
                Ok(file_id) => file_id,
                Err(_) => {
                    return Err(YamlError::InvalidFileId {
                        line: line.number,
                        text: anchor.to_string(),
                    })
                }
            };
            cursor = anchor[id_end..].trim();
        }
        let stripped = cursor == "stripped";
        if !stripped && !cursor.is_empty() {
            return Err(YamlError::InvalidDocumentHeader {
                line: line.number,
                text: line.text.to_string(),
            });
        }
        self.position += 1;
        let root = self.parse_body(line.number)?;
        Ok(Document { class_id, file_id, stripped, root })
    }

    fn parse_body(&mut self, header_line: usize) -> Result<Node, YamlError> {
        let line = match self.current() {
            Some(line) => line,
            None => {
                return Err(YamlError::UnexpectedContent {
                    line: header_line,
                    text: String::from("document body is empty"),
                })
            }
        };
        if line.text.starts_with("---") {
            return Err(YamlError::UnexpectedContent {
                line: line.number,
                text: line.text.to_string(),
            });
        }
        let indent = line.indent;
        self.parse_node(indent)
    }

    fn parse_node(&mut self, indent: usize) -> Result<Node, YamlError> {
        let line = match self.current() {
            Some(line) => line,
            None => return Ok(Node::Scalar(String::new())),
        };
        if line.text == "-" {
            return self.parse_sequence(indent);
        }
        if line.text.starts_with('{') || line.text.starts_with('[') {
            let inline = line.text.to_string();
            self.position += 1;
            return parse_inline(&inline, line.number);
        }
        if find_key_separator(line.text).is_some() {
            return self.parse_mapping(indent);
        }
        self.position += 1;
        Ok(Node::Scalar(line.text.trim().to_string()))
    }

    fn parse_mapping(&mut self, indent: usize) -> Result<Node, YamlError> {
        let mut entries: Vec<(String, Node)> = Vec::new();
        while let Some(line) = self.current() {
            if line.indent < indent {
                break;
            }
            if line.text.starts_with("---") {
                break;
            }
            if line.indent > indent {
                return Err(YamlError::UnexpectedIndent { line: line.number, expected: indent });
            }
            if line.text == "-" {
                break;
            }
            let separator = match find_key_separator(line.text) {
                Some(separator) => separator,
                None => {
                    return Err(YamlError::UnexpectedContent {
                        line: line.number,
                        text: line.text.to_string(),
                    })
                }
            };
            let key = decode_key(line.text[..separator].trim(), line.number)?;
            let inline = line.text[separator + 1..].trim().to_string();
            self.position += 1;
            let value = self.parse_value(indent, &inline, line.number)?;
            entries.push((key, value));
        }
        Ok(Node::Mapping(entries))
    }

    fn parse_sequence(&mut self, indent: usize) -> Result<Node, YamlError> {
        let mut items: Vec<Node> = Vec::new();
        while let Some(line) = self.current() {
            if line.indent != indent || line.text != "-" {
                break;
            }
            self.position += 1;
            let item = match self.current() {
                Some(next) if next.indent > indent && !next.text.starts_with("---") => {
                    self.parse_node(next.indent)?
                }
                _ => Node::Scalar(String::new()),
            };
            items.push(item);
        }
        Ok(Node::Sequence(items))
    }

    fn parse_value(&mut self, indent: usize, inline: &str, line: usize) -> Result<Node, YamlError> {
        if !inline.is_empty() {
            return parse_inline(inline, line);
        }
        match self.current() {
            Some(next) if next.indent > indent && !next.text.starts_with("---") => {
                self.parse_node(next.indent)
            }
            Some(next) if next.indent == indent && next.text == "-" => self.parse_sequence(indent),
            _ => Ok(Node::Scalar(String::new())),
        }
    }
}

fn find_key_separator(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        let byte = match bytes.get(index) {
            Some(byte) => *byte,
            None => break,
        };
        match quote {
            Some(b'"') => {
                if byte == b'\\' {
                    index += 2;
                    continue;
                }
                if byte == b'"' {
                    quote = None;
                }
                index += 1;
            }
            Some(_) => {
                if byte == b'\'' {
                    quote = None;
                }
                index += 1;
            }
            None => {
                if byte == b'"' || byte == b'\'' {
                    quote = Some(byte);
                    index += 1;
                    continue;
                }
                if byte == b':' {
                    match bytes.get(index + 1) {
                        None => return Some(index),
                        Some(b' ') => return Some(index),
                        Some(_) => {}
                    }
                }
                index += 1;
            }
        }
    }
    None
}

fn decode_key(text: &str, line: usize) -> Result<String, YamlError> {
    let trimmed = text.trim();
    match trimmed.chars().next() {
        Some('"') | Some('\'') => {
            let mut flow = Flow::new(trimmed);
            let key = flow.parse_quoted(line)?;
            flow.skip_whitespace();
            if !flow.at_end() {
                return Err(YamlError::UnexpectedContent { line, text: trimmed.to_string() });
            }
            Ok(key)
        }
        _ => Ok(trimmed.to_string()),
    }
}

fn parse_inline(text: &str, line: usize) -> Result<Node, YamlError> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let mut flow = Flow::new(trimmed);
        let node = flow.parse_value(line)?;
        flow.skip_whitespace();
        if !flow.at_end() {
            return Err(YamlError::UnexpectedContent { line, text: trimmed.to_string() });
        }
        Ok(node)
    } else {
        Ok(Node::Scalar(decode_plain_scalar(trimmed, line)?))
    }
}

fn decode_plain_scalar(text: &str, line: usize) -> Result<String, YamlError> {
    match text.chars().next() {
        Some('"') | Some('\'') => {
            let mut flow = Flow::new(text);
            let value = flow.parse_quoted(line)?;
            flow.skip_whitespace();
            if !flow.at_end() {
                return Err(YamlError::UnexpectedContent { line, text: text.to_string() });
            }
            Ok(value)
        }
        _ => Ok(text.to_string()),
    }
}

struct Flow<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Flow<'a> {
    fn new(text: &'a str) -> Self {
        Flow { text, position: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.text[self.position..]
    }

    fn at_end(&self) -> bool {
        self.position >= self.text.len()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(character) = self.peek() {
            if !character.is_whitespace() {
                break;
            }
            self.position += character.len_utf8();
        }
    }

    fn parse_value(&mut self, line: usize) -> Result<Node, YamlError> {
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.parse_mapping(line),
            Some('[') => self.parse_sequence(line),
            Some(_) => Ok(Node::Scalar(self.parse_scalar(line)?)),
            None => Err(YamlError::UnterminatedFlow { line }),
        }
    }

    fn parse_scalar(&mut self, line: usize) -> Result<String, YamlError> {
        match self.peek() {
            Some('"') | Some('\'') => self.parse_quoted(line),
            _ => {
                let rest = self.rest();
                let end = rest.find([',', '}', ']']).unwrap_or(rest.len());
                let value = rest[..end].trim().to_string();
                self.position += end;
                Ok(value)
            }
        }
    }

    fn parse_quoted(&mut self, line: usize) -> Result<String, YamlError> {
        let quote = match self.peek() {
            Some(character) if character == '"' || character == '\'' => character,
            _ => {
                return Err(YamlError::UnexpectedContent { line, text: self.rest().to_string() })
            }
        };
        self.position += quote.len_utf8();
        let mut value = String::new();
        loop {
            let character = match self.rest().chars().next() {
                Some(character) => character,
                None => return Err(YamlError::UnterminatedString { line }),
            };
            self.position += character.len_utf8();
            if character == quote {
                if quote == '\'' && self.rest().starts_with('\'') {
                    self.position += 1;
                    value.push('\'');
                    continue;
                }
                return Ok(value);
            }
            if quote == '"' && character == '\\' {
                let escaped = match self.rest().chars().next() {
                    Some(escaped) => escaped,
                    None => return Err(YamlError::UnterminatedString { line }),
                };
                self.position += escaped.len_utf8();
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '0' => value.push('\0'),
                    'b' => value.push('\u{8}'),
                    'f' => value.push('\u{c}'),
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    '/' => value.push('/'),
                    ' ' => value.push(' '),
                    'u' => {
                        let digits: String = self.rest().chars().take(4).collect();
                        if digits.len() != 4 {
                            return Err(YamlError::InvalidEscape { line, text: digits });
                        }
                        let code = match u32::from_str_radix(&digits, 16) {
                            Ok(code) => code,
                            Err(_) => {
                                return Err(YamlError::InvalidEscape { line, text: digits })
                            }
                        };
                        self.position += 4;
                        match char::from_u32(code) {
                            Some(decoded) => value.push(decoded),
                            None => {
                                return Err(YamlError::InvalidEscape { line, text: digits })
                            }
                        }
                    }
                    other => {
                        value.push('\\');
                        value.push(other);
                    }
                }
                continue;
            }
            value.push(character);
        }
    }

    fn parse_key(&mut self, line: usize) -> Result<String, YamlError> {
        match self.peek() {
            Some('"') | Some('\'') => self.parse_quoted(line),
            _ => {
                let rest = self.rest();
                let end = rest.find(':').unwrap_or(rest.len());
                let key = rest[..end].trim().to_string();
                self.position += end;
                Ok(key)
            }
        }
    }

    fn parse_mapping(&mut self, line: usize) -> Result<Node, YamlError> {
        self.position += 1;
        let mut entries: Vec<(String, Node)> = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('}') => {
                    self.position += 1;
                    break;
                }
                Some(',') => {
                    self.position += 1;
                    continue;
                }
                None => return Err(YamlError::UnterminatedFlow { line }),
                _ => {}
            }
            let key = self.parse_key(line)?;
            self.skip_whitespace();
            match self.peek() {
                Some(':') => self.position += 1,
                None => return Err(YamlError::UnterminatedFlow { line }),
                _ => {
                    return Err(YamlError::UnexpectedContent {
                        line,
                        text: self.rest().to_string(),
                    })
                }
            }
            let value = self.parse_value(line)?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(',') => self.position += 1,
                Some('}') => {
                    self.position += 1;
                    break;
                }
                None => return Err(YamlError::UnterminatedFlow { line }),
                _ => {
                    return Err(YamlError::UnexpectedContent {
                        line,
                        text: self.rest().to_string(),
                    })
                }
            }
        }
        Ok(Node::Mapping(entries))
    }

    fn parse_sequence(&mut self, line: usize) -> Result<Node, YamlError> {
        self.position += 1;
        let mut items: Vec<Node> = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(']') => {
                    self.position += 1;
                    break;
                }
                Some(',') => {
                    self.position += 1;
                    continue;
                }
                None => return Err(YamlError::UnterminatedFlow { line }),
                _ => {}
            }
            let value = self.parse_value(line)?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => self.position += 1,
                Some(']') => {
                    self.position += 1;
                    break;
                }
                None => return Err(YamlError::UnterminatedFlow { line }),
                _ => {
                    return Err(YamlError::UnexpectedContent {
                        line,
                        text: self.rest().to_string(),
                    })
                }
            }
        }
        Ok(Node::Sequence(items))
    }
}
