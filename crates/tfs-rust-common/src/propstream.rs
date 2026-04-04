use crate::error::{Result, TfsRustError};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};

pub struct PropWriteStream {
    buf: Vec<u8>,
}

impl Default for PropWriteStream {
    fn default() -> Self {
        Self::new()
    }
}

impl PropWriteStream {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        let _ = self.buf.write_u16::<LittleEndian>(v);
    }

    pub fn write_u32(&mut self, v: u32) {
        let _ = self.buf.write_u32::<LittleEndian>(v);
    }

    pub fn write_u64(&mut self, v: u64) {
        let _ = self.buf.write_u64::<LittleEndian>(v);
    }

    pub fn write_string(&mut self, s: &str) {
        let len = s.len() as u16;
        self.write_u16(len);
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}

pub struct PropStream<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> PropStream<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(data),
        }
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.cursor
            .read_u8()
            .map_err(|_| TfsRustError::PropStream("EOF reading u8".into()))
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        self.cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| TfsRustError::PropStream("EOF reading u16".into()))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        self.cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| TfsRustError::PropStream("EOF reading u32".into()))
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        self.cursor
            .read_u64::<LittleEndian>()
            .map_err(|_| TfsRustError::PropStream("EOF reading u64".into()))
    }

    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_u16()? as usize;
        let mut string_buf = vec![0; len];
        self.cursor
            .read_exact(&mut string_buf)
            .map_err(|_| TfsRustError::PropStream("EOF reading string".into()))?;
        String::from_utf8(string_buf)
            .map_err(|_| TfsRustError::PropStream("Invalid UTF-8 in string".into()))
    }
}
