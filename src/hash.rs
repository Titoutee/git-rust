use sha1::{Digest, Sha1};
use std::io::prelude::*;

pub struct HashWriter<W> {
    pub writer: W,
    pub hasher: Sha1,
}

impl<W: Write> Write for HashWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[1..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
