//! Everything related to ANSI output - styling, commands, colors

pub mod ansi;
pub mod color;
pub mod command;
pub(crate) mod io;
pub mod style;

/// The standard output writer behind [`Terminal::try_system_default`].
///
/// An opaque stand-in so the terminal's type can be named without committing
/// to `std::io::Stdout`. Deliberately unbuffered: [`Termit`] wraps whatever
/// output it is handed in an 8K `BufWriter`, so buffering here would just
/// copy every byte twice.
///
/// [`Terminal::try_system_default`]: crate::Terminal::try_system_default
/// [`Termit`]: crate::Termit
pub struct DefaultOutput(std::io::Stdout);

impl Default for DefaultOutput {
    fn default() -> Self {
        Self(std::io::stdout())
    }
}

impl std::io::Write for DefaultOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.0.write_all(buf)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.0.write_fmt(args)
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}
