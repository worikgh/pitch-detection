//! `RxProxy` is a simple class to allow the reuse of a
//! `mpsc::Receiver`.  The receiver to reuse (rx_one` is passed in the
//! constructor `new`.  To reuse `rx_one` create a channel pair using
//! `(tx, rx) = mpsc::Channel<T>()` and pass `tx` to `set_sender`.
//! Any messages received on `rx_one` will be sent to `tx` ad can be
//! received on the new `rx`.  `set_sender` can be called any number
//! of times, and it overwrites the previous setting each time.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct RxProxy<T>
where
    T: Send + 'static,
{
    opt_tx: Arc<Mutex<Option<Sender<T>>>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl<T> RxProxy<T>
where
    T: Send + 'static,
{
    /// Stop the proxy
    pub fn stop(&mut self) {
        if self.running.swap(false, Ordering::SeqCst) {
            // Wait for thread to finish
            if let Some(h) = self.thread_handle.take() {
                _ = h.join();
            }
        }
    }

    /// Check the proxy
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the proxy
    fn run(
        rx: Receiver<T>,
        opt_tx: Arc<Mutex<Option<Sender<T>>>>,
        running: Arc<AtomicBool>,
    ) -> Result<JoinHandle<()>, Box<dyn std::error::Error>> {
        if running.swap(true, Ordering::SeqCst) {
            return Err("Already running".into());
        }
        // let rx = self.rx.clone();

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                // Clone the sender.  Multiple senders are allowed
                let current_tx = { opt_tx.lock().unwrap().clone() };

                // Hold the receiver lock and loop receiving until the
                // channels change
                // let rx_guard = rx.lock().unwrap();

                loop {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    let message = {
                        match rx.recv_timeout(Duration::from_millis(100)) {
                            Ok(m) => Some(m),
                            Err(RecvTimeoutError::Timeout) => None,
                            Err(RecvTimeoutError::Disconnected) => {
                                // Channel disconnected, stop the proxy
                                running.store(false, Ordering::SeqCst);
                                break;
                            }
                        }
                    };
                    // If there is a sender send the message,
                    // otherwise drop it
                    if let Some(sender) = &current_tx {
                        if let Some(msg) = message {
                            if sender.send(msg).is_err() {
                                // Clear the sender on error
                                *opt_tx.lock().unwrap() = None;
                                break;
                            }
                        }
                    }
                }
            }
        });
        Ok(handle)
    }

    /// `rx` is the receiver to proxy
    pub fn new(rx: Receiver<T>) -> Self {
        let opt_tx = Arc::new(Mutex::new(None));
        let running = Arc::new(AtomicBool::new(false));
        let thread_handle = match Self::run(rx, opt_tx.clone(), running.clone()) {
            Ok(h) => h,
            Err(err) => panic!("RxProxy::new: Starting thread failed; {err}"),
        };
        Self {
            opt_tx,
            running,
            thread_handle: Some(thread_handle),
        }
    }

    /// Provide the sending end of a channel to proxy messages on `rx`
    pub fn set_sender(&mut self, tx: Sender<T>) {
        *self.opt_tx.lock().unwrap() = Some(tx);
    }
}

// Add the same trait bounds to the Drop implementation
impl<T> Drop for RxProxy<T>
where
    T: Send + 'static,
{
    fn drop(&mut self) {
        self.stop();
    }
}
