//! Subprocess transport managing language server lifecycle and framed stdio pipes.

use std::io::{self, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Serialize;

use crate::jsonrpc::{encode_payload, incoming_message, read_message};

/// Manages stdio pipes and child process lifecycle for a language server.
pub struct transport {
    stdin: Arc<Mutex<ChildStdin>>,
    child: Arc<Mutex<Child>>,
    is_running: Arc<AtomicBool>,
}

impl transport {
    /// Spawns a language server subprocess and begins asynchronous stdio communication.
    ///
    /// Returns the transport handle and a receiver channel for incoming framed messages.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the subprocess fails to spawn or if stdio cannot be piped.
    pub fn spawn(
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
    ) -> io::Result<(Self, Receiver<incoming_message>)> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "failed to open server stdin pipe",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "failed to open server stdout pipe",
            )
        })?;

        let (tx, rx): (Sender<incoming_message>, Receiver<incoming_message>) = mpsc::channel();
        let is_running = Arc::new(AtomicBool::new(true));

        let is_running_reader = Arc::clone(&is_running);
        thread::Builder::new()
            .name("lsp-reader".to_string())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                while is_running_reader.load(Ordering::Relaxed) {
                    match read_message(&mut reader) {
                        Ok(Some(msg)) => {
                            if tx.send(msg).is_err() {
                                break;
                            }
                        }
                        Ok(None) => break, // EOF reached
                        Err(_) => break,
                    }
                }
                is_running_reader.store(false, Ordering::Relaxed);
            })?;

        Ok((
            Self {
                stdin: Arc::new(Mutex::new(stdin)),
                child: Arc::new(Mutex::new(child)),
                is_running,
            },
            rx,
        ))
    }

    /// Sends a serializable message over the framed stdio connection.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if serialization or writing to stdin fails.
    pub fn send<T: Serialize>(&self, msg: &T) -> io::Result<()> {
        let bytes = encode_payload(msg).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to serialize LSP payload: {e}"),
            )
        })?;

        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "poisoned stdin lock"))?;
        stdin.write_all(&bytes)?;
        stdin.flush()?;
        Ok(())
    }

    /// Checks if the language server subprocess is still actively running.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        if !self.is_running.load(Ordering::Relaxed) {
            return false;
        }
        if let Ok(mut child) = self.child.lock() {
            match child.try_wait() {
                Ok(None) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Terminates the subprocess.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if terminating the process fails.
    pub fn terminate(&self) -> io::Result<()> {
        self.is_running.store(false, Ordering::Relaxed);
        let mut child = self
            .child
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "poisoned child lock"))?;
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    }
}

impl Drop for transport {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
