use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cell::RefCell,
    fmt::{Debug, Formatter},
};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct FlowSnake(pub u64);

thread_local! {
    static last_generation_time: RefCell<u64> = RefCell::new(0);
    static seq_number: RefCell<u64> = RefCell::new(0);
    static loc_worker_id: RefCell<once_cell::unsync::Lazy<u64>> = RefCell::new(
        once_cell::unsync::Lazy::new(get_worker_id)
    );
}

fn get_worker_id() -> u64 {
    rand::random::<u64>()
}

pub const timestamp_bits: u32 = 34;
pub const worker_id_bits: u32 = 12;
pub const sequence_bits: u32 = 18;

const CHAR_TO_BASE32: [u8; 128] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 255, 255, 255,
    255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 16, 17, 255, 18, 19, 255, 20, 21, 255, 22, 23, 24,
    25, 26, 255, 27, 28, 29, 30, 31, 255, 255, 255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 16, 17,
    255, 18, 19, 255, 20, 21, 255, 22, 23, 24, 25, 26, 255, 27, 28, 29, 30, 31, 255, 255, 255, 255,
    255,
];

impl FlowSnake {
    pub fn new_parts(timestamp: u64, worker_id: u64, seq: u64) -> FlowSnake {
        let n = ((timestamp & ((1 << timestamp_bits) - 1)) << (worker_id_bits + sequence_bits))
            | ((worker_id & ((1 << worker_id_bits) - 1)) << (sequence_bits))
            | (seq & ((1 << sequence_bits) - 1));
        FlowSnake(n)
    }

    pub fn generate() -> FlowSnake {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let worker_id = loc_worker_id.with(|x| **x.borrow_mut());
        let seq = if last_generation_time.with(|x| time <= *x.borrow()) {
            seq_number.with(|s| {
                let mut s = s.borrow_mut();
                let seq = *s;
                *s += 1;
                seq
            })
        } else {
            last_generation_time.with(|t| *(t.borrow_mut()) = time);
            let rnd = rand::random::<u64>() % ((1 << sequence_bits) - (1 << (sequence_bits - 2)));
            seq_number.with(|s| {
                (*s.borrow_mut()) = rnd + 1;
            });
            rnd
        };

        FlowSnake::new_parts(time, worker_id, seq)
    }

    pub fn parse(s: &str) -> Result<FlowSnake, ()> {
        if s.len() != 13 {
            return Err(());
        }
        let mut n = 0u64;
        for ch in s.chars() {
            let ch = ch as usize;
            if ch >= CHAR_TO_BASE32.len() {
                return Err(());
            }
            let five_bit = CHAR_TO_BASE32[ch] as u64;
            if five_bit == 255 {
                return Err(());
            }
            n <<= 5;
            n |= five_bit;
        }
        Ok(FlowSnake(n))
    }

    pub fn write_str_buffered(&self, buf: &mut [u8]) -> Result<(), ()> {
        if buf.len() < 13 {
            return Err(());
        }
        for i in 0..13 {
            let x = ((self.0 >> (5 * (12 - i))) & 0xff) as u8;
            buf[i as usize] = x;
        }
        Ok(())
    }
}

impl std::fmt::Display for FlowSnake {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = [0u8; 13];
        self.write_str_buffered(&mut s).unwrap();
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
            Err(e) => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"A 13-byte base32 string",
            )),
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
