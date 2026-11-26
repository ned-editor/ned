//! counting writes wrapper for terminal output tracking.

use std::io;

pub(crate) struct CountingWrites<IO> {
    pub inner: IO,
    pub count: usize,
    pub busy: bool,
}

impl<IO> io::Write for CountingWrites<IO>
where
    IO: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner.write(buf) {
            Ok(len) => {
                self.busy = true;
                self.count += len;
                Ok(len)
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.busy = false;
        self.inner.flush()
    }
}

impl<IO> CountingWrites<IO> {
    pub(crate) fn new(io: IO) -> Self {
        Self {
            inner: io,
            count: 0,
            busy: false,
        }
    }
}
