//! counting writes wrapper for terminal output tracking.

use std::io;

/// Wrapper around an IO writer that tracks bytes written and busy state.
pub(crate) struct CountingWrites<IO> {
    /// Inner IO writer.
    pub inner: IO,
    /// Total number of bytes written.
    pub count: usize,
    /// Whether any write has occurred.
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
    /// Creates a new `CountingWrites` wrapping the given IO writer.
    pub(crate) fn new(io: IO) -> Self {
        Self {
            inner: io,
            count: 0,
            busy: false,
        }
    }
}
