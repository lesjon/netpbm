//! module for parsing netpbm images
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::ErrorKind;
use std::ops::{AddAssign, ShlAssign};
use log;

// Type         	Magic number	    Extension	Colors
//                  ASCII (plain)	Binary (raw)
// Portable BitMap	P1	            P4	.pbm	0–1 (white & black)
// Portable GrayMap	P2	            P5	.pgm	0–255 (gray scale), 0–65535 (gray scale), variable, black-to-white range
// Portable PixMap	P3 	            P6	.ppm	16777216 (0–255 for each RGB channel), some support for 0-65535 per channel
// A value of P7 refers to the PAM file format that is covered as well by the netpbm library.[9]
pub mod magic_numbers {
    pub const PBM_ASCII: &[u8] = b"P1";
    pub const PGM_ASCII: &[u8] = b"P2";
    pub const PPM_ASCII: &[u8] = b"P3";
    pub const PBM_BINARY: &[u8] = b"P4";
    pub const PGM_BINARY: &[u8] = b"P5";
    pub const PPM_BINARY: &[u8] = b"P6";
    pub const PAM_BINARY: &[u8] = b"P7";
}

#[derive(Clone)]
pub struct Image<F: Clone> {
    pub data: Vec<F>,
    pub width: usize,
    pub height: usize,
    pub max_value: u16,
}

#[derive(Debug, PartialEq)]
enum PgmParseState {
    Type,
    Width,
    Height,
    MaxValue,
    Data,
}

// PGM uses 8 or 16 bits per pixel
const WHITESPACE_BYTES: &[u8] = b" \n\r\t";

struct BytesParser {
    whitespace_index: usize,
    prev_whitespace_index: usize,
}

impl BytesParser {
    fn new() -> BytesParser {
        log::debug!("Created new BytesParser");
        BytesParser {
            whitespace_index: 0,
            prev_whitespace_index: 0,
        }
    }

    fn take_line<'a>(&mut self, contents: &'a [u8]) -> Result<&'a [u8], Box<dyn Error>> {
        let len = contents.len();
        log::debug!("take line from contents with size{len}");
        let searchable = &contents[self.whitespace_index..];
        self.prev_whitespace_index = self.whitespace_index;
        if let Some(i) = searchable.iter().position(|byte| WHITESPACE_BYTES.contains(byte)) {
            self.whitespace_index += i + 1;
        }
        let line = &contents[self.prev_whitespace_index..self.whitespace_index - 1];
        log::debug!("Found line '{line:?}'");
        Ok(line)
    }

    pub fn take_rest<'a>(&mut self, contents: &'a [u8]) -> &'a [u8] {
        self.prev_whitespace_index = self.whitespace_index;
        self.whitespace_index = contents.len();
        &contents[self.prev_whitespace_index..]
    }
}

fn parse_usize(bytes: &[u8]) -> Result<usize, Box<dyn Error>> {
    log::debug!("parse_usize('{bytes:?}')");
    let mut result = 0usize;
    for byte in bytes {
        result *= 10;
        result.add_assign((*byte - b'0') as usize);
    }
    log::debug!("result {result}");
    Ok(result)
}

fn parse_u16(bytes: &[u8]) -> Result<u16, Box<dyn Error>> {
    log::debug!("parse_u16('{bytes:?}')");
    let mut result = 0u16;
    for byte in bytes {
        result *= 10;
        result.add_assign((*byte - b'0') as u16);
    }
    log::debug!("result {result}");
    Ok(result)
}

pub fn parse(contents: &[u8]) -> Result<Image<u16>, Box<dyn Error>> {
    log::info!("start parsing contents of size {}", contents.len());
    let mut data = vec![];
    let mut parse_state = PgmParseState::Type;
    let mut width = None;
    let mut height = None;
    let mut max_value = None;
    let mut pgm_type = None;
    let mut bytes_parser = BytesParser::new();
    loop {
        let line = if parse_state != PgmParseState::Data { bytes_parser.take_line(contents)? } else { bytes_parser.take_rest(contents) };
        log::debug!("loop: line='{:?}'", &line[..usize::min(line.len(), 10)]);
        if line.is_empty() {
            continue;
        }
        if line.starts_with(b"#") {
            continue;
        }

        parse_state = match parse_state {
            PgmParseState::Type => {
                log::debug!("Parsing p*m type");
                pgm_type = Some(line);
                PgmParseState::Width
            }
            PgmParseState::Width => {
                log::debug!("parsing width");
                width = Some(parse_usize(line)?);
                PgmParseState::Height
            }
            PgmParseState::Height => {
                log::debug!("Parsing height");
                height = Some(parse_usize(line)?);
                PgmParseState::MaxValue
            }
            PgmParseState::MaxValue => {
                log::debug!("Parsing max_value");
                max_value = Some(parse_u16(line)?);
                PgmParseState::Data
            }
            PgmParseState::Data => {
                log::debug!("Parsing data with type '{:?}'; data has length:{}", pgm_type, line.len());
                match pgm_type.unwrap() {
                    magic_numbers::PGM_BINARY => {
                        if max_value.unwrap() > 255 {
                            for (i, byte) in line.iter().enumerate() {
                                if i % 2 == 0 {
                                    data.push(*byte as u16)
                                } else {
                                    if let Some(last) = data.last_mut() {
                                        last.shl_assign(8);
                                        *last += *byte as u16;
                                    }
                                }
                            }
                        } else {
                            data.extend(line.iter().map(|b| *b as u16));
                        }
                    }
                    magic_numbers::PGM_ASCII => {
                        while let Ok(some) = bytes_parser.take_line(line) {
                            if some.is_empty() {
                                // double white characters will give empty lines, skip them
                                continue;
                            }
                            match parse_u16(some) {
                                Ok(val) => data.push(val),
                                Err(e) => {
                                    log::warn!("Finished parsing ASCII data on Error:{e}");
                                    break;
                                }  // assume that an error means the end of the ascii data
                            }
                        }
                    }
                    magic_numbers::PBM_ASCII => { todo!() }
                    magic_numbers::PBM_BINARY => { todo!() }
                    magic_numbers::PPM_ASCII => { todo!() }
                    magic_numbers::PPM_BINARY => { todo!() }
                    magic_numbers::PAM_BINARY => { todo!() }
                    &_ => return Err(Box::new(io::Error::new(ErrorKind::Unsupported, "Unkown image format!"))),
                }
                break;
            }
        }
    }
    let max_value = max_value.expect("Did not get max_value from PGM file");
    let width = width.expect("Did not get width from PGM file");
    let height = height.expect("Did not get height from PGM file");
    Ok(Image {
        data,
        width,
        height,
        max_value,
    })
}

impl Display for Image<u16> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Image { data, width, height, max_value } => {
                log::debug!("Display image with size:({},{})", width, height);
                for row in 0..*height {
                    for col in 0..*width {
                        let gray = f32::from(data[row * width + col]) / f32::from(*max_value);
                        let char = if gray < 0.2 {
                            ' '
                        } else if gray < 0.4 {
                            '░'
                        } else if gray < 0.6 {
                            '▒'
                        } else if gray < 0.8 {
                            '▓'
                        } else {
                            '█'
                        };
                        write!(f, "{char}")?;
                    }
                    writeln!(f)?;
                }
                write!(f, "")
            }
        }
    }
}
