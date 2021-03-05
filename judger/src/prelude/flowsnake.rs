use err_derive::Error;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cell::RefCell,
    fmt::{Debug, Formatter},
};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct FlowSnake(pub u64);

thread_local! {
    static LAST_GENERATION_TIME: RefCell<u64> = RefCell::new(0);
    static SEQ_NUMBER: RefCell<u64> = RefCell::new(0);
    static LOC_WORKER_ID: RefCell<once_cell::unsync::Lazy<u64>> = RefCell::new(
        once_cell::unsync::Lazy::new(get_worker_id)
    );
}

fn get_worker_id() -> u64 {
    rand::random::<u64>()
}

pub const TIMESTAMP_BITS: u32 = 34;
pub const WORKER_ID_BITS: u32 = 12;
pub const SEQUENCE_BITS: u32 = 18;

const CHAR_TO_BASE32: [u8; 128] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 255, 255, 255,
    255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 16, 17, 255, 18, 19, 255, 20, 21, 255, 22, 23, 24,
    25, 26, 255, 27, 28, 29, 30, 31, 255, 255, 255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 16, 17,
    255, 18, 19, 255, 20, 21, 255, 22, 23, 24, 25, 26, 255, 27, 28, 29, 30, 31, 255, 255, 255, 255,
    255,
];

const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

pub enum FlowSnakeDeserializeError {
    InvalidLength(usize),
    InvalidChar(usize, char),
}

impl FlowSnake {
    pub fn new_parts(timestamp: u64, worker_id: u64, seq: u64) -> FlowSnake {
        let n = ((timestamp & ((1 << TIMESTAMP_BITS) - 1)) << (WORKER_ID_BITS + SEQUENCE_BITS))
            | ((worker_id & ((1 << WORKER_ID_BITS) - 1)) << (SEQUENCE_BITS))
            | (seq & ((1 << SEQUENCE_BITS) - 1));
        FlowSnake(n)
    }

    pub fn generate() -> FlowSnake {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let worker_id = LOC_WORKER_ID.with(|x| **x.borrow_mut());
        let seq = if LAST_GENERATION_TIME.with(|x| time <= *x.borrow()) {
            SEQ_NUMBER.with(|s| {
                let mut s = s.borrow_mut();
                let seq = *s;
                *s += 1;
                seq
            })
        } else {
            LAST_GENERATION_TIME.with(|t| *(t.borrow_mut()) = time);
            let rnd = rand::random::<u64>() % ((1 << SEQUENCE_BITS) - (1 << (SEQUENCE_BITS - 2)));
            SEQ_NUMBER.with(|s| {
                (*s.borrow_mut()) = rnd + 1;
            });
            rnd
        };

        FlowSnake::new_parts(time, worker_id, seq)
    }

    pub fn parse(s: &str) -> Result<FlowSnake, FlowSnakeDeserializeError> {
        if s.len() < 13 {
            return Err(FlowSnakeDeserializeError::InvalidLength(s.len()));
        }
        let mut n = 0u64;
        for (pos, ch_) in s.chars().filter(|x| *x != '-').enumerate() {
            let ch = ch_ as usize;
            if ch >= CHAR_TO_BASE32.len() {
                return Err(FlowSnakeDeserializeError::InvalidChar(pos, ch_));
            }
            let five_bit = CHAR_TO_BASE32[ch] as u64;
            if five_bit == 255 {
                return Err(FlowSnakeDeserializeError::InvalidChar(pos, ch_));
            }
            n <<= 5;
            n |= five_bit;
        }
        Ok(FlowSnake(n))
    }

    pub fn write_str_buffered(&self, buf: &mut [u8]) -> Result<(), FlowSnakeFormatErr> {
        if buf.len() < 13 {
            return Err(FlowSnakeFormatErr::SliceTooSmall);
        }
        for i in 0..13 {
            let x = ((self.0 >> (5 * (12 - i))) & 31) as u8;
            buf[i as usize] = ALPHABET[x as usize];
        }
        Ok(())
    }

    pub fn write_str_dashed_buffered(&self, buf: &mut [u8]) -> Result<(), FlowSnakeFormatErr> {
        if buf.len() < 14 {
            return Err(FlowSnakeFormatErr::SliceTooSmall);
        }
        for i in 0..7 {
            let x = ((self.0 >> (5 * (12 - i))) & 31) as u8;
            buf[i as usize] = ALPHABET[x as usize];
        }
        buf[7] = b'-';
        for i in 7..13 {
            let x = ((self.0 >> (5 * (12 - i))) & 31) as u8;
            buf[(i + 1) as usize] = ALPHABET[x as usize];
        }
        Ok(())
    }
}

impl From<u64> for FlowSnake {
    fn from(i: u64) -> Self {
        FlowSnake(i)
    }
}

impl std::fmt::Display for FlowSnake {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = [0u8; 14];
        self.write_str_dashed_buffered(&mut s).unwrap();
        let s = unsafe { std::str::from_utf8_unchecked(&s) };
        f.write_str(s)
    }
}

impl Debug for FlowSnake {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Serialize for FlowSnake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = [0u8; 13];
        self.write_str_buffered(&mut st).unwrap();
        let st = unsafe { std::str::from_utf8_unchecked(&st) };
        serializer.serialize_str(st)
    }
}

#[derive(Debug, Error)]
pub enum FlowSnakeFormatErr {
    #[error(display = "not enough space to format")]
    SliceTooSmall,
}

struct FlowSnakeVisitor;

impl<'de> Visitor<'de> for FlowSnakeVisitor {
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlowSnake(v as u64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlowSnake(v as u64))
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match FlowSnake::parse(v) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                FlowSnakeDeserializeError::InvalidLength(len) => {
                    Err(serde::de::Error::invalid_length(len, &"13"))
                }
                FlowSnakeDeserializeError::InvalidChar(pos, ch) => Err(serde::de::Error::custom(
                    format!("Invalid character `{}` at position {}", ch, pos),
                )),
            },
        }
    }

    type Value = FlowSnake;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("A 13-byte base32 string")
    }
}

impl<'de> Deserialize<'de> for FlowSnake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(FlowSnakeVisitor)
    }
}
