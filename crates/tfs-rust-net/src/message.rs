use bytes::{BufMut, BytesMut};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use tfs_rust_common::{
    error::{Result, TfsRustError},
    Position,
};

pub struct NetworkMessage {
    pub buf: BytesMut,
    pub read_pos: usize,
}

impl Default for NetworkMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkMessage {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::new(),
            read_pos: 0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut buf = BytesMut::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        Self { buf, read_pos: 0 }
    }

    pub fn unread_bytes(&self) -> usize {
        self.buf.len().saturating_sub(self.read_pos)
    }

    /// Full buffer (including consumed prefix); used after writing outgoing payload.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Take owned copy of the full buffer (for enqueueing on the game thread).
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf.to_vec()
    }

    pub fn skip(&mut self, offset: usize) -> Result<()> {
        if self.unread_bytes() < offset {
            return Err(TfsRustError::Protocol("EOF".into()));
        }
        self.read_pos += offset;
        Ok(())
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        if self.unread_bytes() < 1 {
            return Err(TfsRustError::Protocol("EOF reading u8".into()));
        }
        let val = self.buf[self.read_pos];
        self.read_pos += 1;
        Ok(val)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        if self.unread_bytes() < 2 {
            return Err(TfsRustError::Protocol("EOF reading u16".into()));
        }
        let val = u16::from_le_bytes([self.buf[self.read_pos], self.buf[self.read_pos + 1]]);
        self.read_pos += 2;
        Ok(val)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        if self.unread_bytes() < 4 {
            return Err(TfsRustError::Protocol("EOF reading u32".into()));
        }
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.buf[self.read_pos..self.read_pos + 4]);
        let val = u32::from_le_bytes(b);
        self.read_pos += 4;
        Ok(val)
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        if self.unread_bytes() < 8 {
            return Err(TfsRustError::Protocol("EOF reading u64".into()));
        }
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.buf[self.read_pos..self.read_pos + 8]);
        let val = u64::from_le_bytes(b);
        self.read_pos += 8;
        Ok(val)
    }

    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_u16()? as usize;
        if self.unread_bytes() < len {
            return Err(TfsRustError::Protocol("EOF reading string".into()));
        }
        let s = std::str::from_utf8(&self.buf[self.read_pos..self.read_pos + len])
            .map_err(|_| TfsRustError::Protocol("Invalid UTF-8 in string".into()))?
            .to_string();
        self.read_pos += len;
        Ok(s)
    }

    pub fn read_position(&mut self) -> Result<Position> {
        let x = self.read_u16()?;
        let y = self.read_u16()?;
        let z = self.read_u8()?;
        Ok(Position::new(x, y, z))
    }

    pub fn write_u8(&mut self, val: u8) {
        self.buf.put_u8(val);
    }

    pub fn write_u16(&mut self, val: u16) {
        self.buf.put_u16_le(val);
    }

    pub fn write_u32(&mut self, val: u32) {
        self.buf.put_u32_le(val);
    }

    pub fn write_u64(&mut self, val: u64) {
        self.buf.put_u64_le(val);
    }

    pub fn write_string(&mut self, val: &str) {
        self.write_u16(val.len() as u16);
        self.buf.extend_from_slice(val.as_bytes());
    }

    pub fn write_position(&mut self, val: &Position) {
        self.write_u16(val.x);
        self.write_u16(val.y);
        self.write_u8(val.z);
    }

    /// `NetworkMessage::addDouble` (`src/networkmessage.cpp`).
    pub fn write_double_tfs(&mut self, value: f64, precision: u8) {
        self.write_u8(precision);
        let scaled = (value * 10_f64.powi(precision as i32)) as i64 + i32::MAX as i64;
        self.write_u32(scaled as u32);
    }

    pub fn decompress(&mut self) -> Result<()> {
        let mut decoder = ZlibDecoder::new(&self.buf[self.read_pos..]);
        let mut uncompressed = Vec::new();
        decoder
            .read_to_end(&mut uncompressed)
            .map_err(|_| TfsRustError::Protocol("Decompression failed".to_string()))?;

        let mut new_buf = BytesMut::with_capacity(self.read_pos + uncompressed.len());
        new_buf.extend_from_slice(&self.buf[..self.read_pos]);
        new_buf.extend_from_slice(&uncompressed);
        self.buf = new_buf;
        Ok(())
    }

    pub fn compress(&mut self) -> Result<()> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&self.buf[self.read_pos..])
            .map_err(|_| TfsRustError::Protocol("Compression failed".to_string()))?;
        let compressed = encoder
            .finish()
            .map_err(|_| TfsRustError::Protocol("Compression failed".to_string()))?;

        let mut new_buf = BytesMut::with_capacity(self.read_pos + compressed.len());
        new_buf.extend_from_slice(&self.buf[..self.read_pos]);
        new_buf.extend_from_slice(&compressed);
        self.buf = new_buf;
        Ok(())
    }
}
