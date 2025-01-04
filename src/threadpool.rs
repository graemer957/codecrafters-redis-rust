use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
};

// Taken straight from the Rust Book
// See: https://doc.rust-lang.org/book/ch20-02-multithreaded.html)
type Job = Box<dyn FnOnce() + Send + 'static>;

#[allow(dead_code)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
}

#[allow(dead_code)]
struct Worker {
    id: usize,
    join_handle: JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let join_handle = std::thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            job();
        });

        Self { id, join_handle }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{thread, time::Duration};

    #[test]
    fn panic_when_zero_size() {
        let result = std::panic::catch_unwind(|| ThreadPool::new(0));
        assert!(result.is_err());
    }

    #[test]
    fn it_works() {
        let (sender, receiver) = mpsc::channel::<()>();
        let pool = ThreadPool::new(1);
        pool.execute(move || {
            let _ = sender.send(());
        });

        thread::sleep(Duration::from_millis(16));

        assert!(receiver.try_recv().is_ok());
    }
}
