//! the main termit application facilitator.

use crate::geometry::Window;
use crate::input::events::{AppEvent, Event};
use crate::input::InputWatcher;
use crate::output::command::{
    CaptureFocus, CaptureMouse, EnterRawMode, Flush, ResetStyles, ShowCursor, TerminalCommander,
    UseAlternateScreen, WrapLines,
};
use crate::output::io::CountingWrites;
use crate::sys::Control;
use crate::widget::Widget;
use crate::{Painter, Screen, Terminal};
use std::io::{self, BufWriter};
use std::marker::PhantomData;

/// the terminal application facilitator managing screen rendering, diffing, and yokio input watching.
pub struct Termit<'a, A: AppEvent> {
    control: Box<dyn Control + 'a>,
    out: Box<dyn io::Write + 'a>,
    screen: Screen,
    painter: Painter,
    /// Whether input stream has closed.
    pub input_closed: bool,
    _marker: PhantomData<A>,
}

impl<'a, A: AppEvent> Termit<'a, A> {
    /// creates a new termit instance with specified control and output writer.
    pub fn new(control: impl Control + 'a, out: Box<dyn io::Write + 'a>) -> Self {
        let size = control.get_size().unwrap_or_default();
        let painter = Painter::default().with_scope(size.window());
        let mut screen = Screen::new(size);
        screen.wipe = true;

        let out_control = control.clone();
        let out = Box::new(BufWriter::with_capacity(
            1024 * 8,
            TerminalResetOnDrop(out_control, out),
        ));

        Self {
            control: Box::new(control),
            out,
            screen,
            painter,
            input_closed: false,
            _marker: PhantomData,
        }
    }

    /// sets whether to draw over existing terminal content.
    #[must_use]
    pub fn draw_over(mut self, draw_over: bool) -> Self {
        self.screen.wipe = !draw_over;
        self
    }

    /// configures 24-bit truecolor support.
    #[must_use]
    pub fn true_color(mut self, true_color: bool) -> Self {
        self.screen.true_color = true_color;
        self
    }

    /// returns mutable reference to the underlying screen buffer.
    pub fn screen_mut(&mut self) -> &mut Screen {
        &mut self.screen
    }

    /// prints ui updates to the terminal using double-buffered diffing.
    ///
    /// # errors
    /// returns `io::Error` if terminal writing fails.
    pub fn print(&mut self, scope: Option<Window>) -> io::Result<()> {
        let mut out = CountingWrites::new(self.out.as_mut());
        let commander = (self.control.as_ref(), &mut out);
        self.screen.print(scope, commander)?;
        if out.busy {
            self.out.flush()?;
        }
        Ok(())
    }

    /// handles input events, updating terminal size if resized and dispatching to widget tree.
    pub fn update<M>(&mut self, model: &mut M, input: Event<A>, root: &mut impl Widget<M, A>) {
        if let Event::Resize(new_size) = input {
            if self.screen.size() != new_size {
                self.screen.resize(new_size);
            }
            self.painter = self.painter.with_scope(new_size.window());
        }
        root.update(model, &input, &mut self.screen, &self.painter);
    }

    /// starts watching terminal input using yokio's non-blocking watcher.
    ///
    /// # errors
    /// returns `io::Error` if watcher registration fails.
    pub fn watch_input<F: FnMut(io::Result<Event<A>>) + 'static>(
        &self,
        on_event: F,
    ) -> io::Result<InputWatcher> {
        crate::input::watch_terminal_input(self.control.as_ref(), self.control.is_raw(), on_event)
    }
}

impl<'a, A: AppEvent, C: Control + 'a, I: 'a, O: io::Write + 'a> From<Terminal<C, I, O>>
    for Termit<'a, A>
{
    fn from(terminal: Terminal<C, I, O>) -> Self {
        let Terminal {
            control,
            output: out,
            ..
        } = terminal;
        Self::new(control, Box::new(out))
    }
}

struct TerminalResetOnDrop<C: Control, O: io::Write>(C, O);

impl<C: Control, O: io::Write> io::Write for TerminalResetOnDrop<C, O> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.1.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.1.flush()
    }
}

impl<C: Control, O: io::Write> Drop for TerminalResetOnDrop<C, O> {
    fn drop(&mut self) {
        let mut commander = (&self.0, &mut self.1);
        let _ = commander.send(ResetStyles);
        let _ = commander.send(UseAlternateScreen(false));
        let _ = commander.send(ShowCursor(true));
        let _ = commander.send(WrapLines(true));
        let _ = commander.send(CaptureMouse(false));
        let _ = commander.send(CaptureFocus(false));
        let _ = commander.send(EnterRawMode);
        let _ = commander.send(Flush);
    }
}
