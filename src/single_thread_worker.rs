// To make this module public, add `pub mod single_thread_worker;` to your `lib.rs`.

use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

/// A type alias for a job to be executed by the worker.
/// The job is a heap-allocated closure that is `Send`-able and can be run only once.
pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// A handle to a single-threaded worker pool.
/// It allows sending jobs to the worker thread.
///
/// This ensures that any function/closure sent via `execute` will run on the same, single thread.
#[derive(Clone)]
pub struct SingleThreadWorker {
    sender: Sender<Job>,
}

impl SingleThreadWorker {
    /// Creates a new `SingleThreadWorker` and spawns a worker thread.
    ///
    /// The worker thread will execute jobs sent to it in the order they are received.
    /// The thread will shut down when the `SingleThreadWorker` and all its clones are dropped.
    ///
    /// # Returns
    ///
    /// A tuple containing the `SingleThreadWorker` handle and the `JoinHandle` for the worker thread.
    /// The `JoinHandle` can be used to wait for the worker thread to finish.
    pub fn new() -> (Self, JoinHandle<()>) {
        let (sender, receiver) = mpsc::channel::<Job>();

        let handle = thread::spawn(move || {
            // The receiver will block until a job is available.
            // When the sender (and all its clones) are dropped, `recv()` will
            // return an `Err`, and the loop will terminate, ending the thread.
            for job in receiver {
                job();
            }
        });

        (Self { sender }, handle)
    }

    /// Sends a job to be executed by the worker thread.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure to be executed. It must be `Send` and have a `'static` lifetime.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // The send can fail if the receiver has been dropped, which would mean the
        // worker thread has panicked. We panic here to propagate the error.
        self.sender
            .send(Box::new(f))
            .expect("Worker thread has panicked");
    }
}
