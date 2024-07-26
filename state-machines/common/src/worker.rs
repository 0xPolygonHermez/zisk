use std::{
    sync::{mpsc::Sender, Mutex},
    thread,
};

pub enum WorkerTask<T> {
    Prove(Box<Vec<T>>),
    Finish,
}

pub struct WorkerHandler<T> {
    tx: Sender<WorkerTask<T>>,
    worker_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl<T> WorkerHandler<T> {
    pub fn new(tx: Sender<WorkerTask<T>>, worker_handle: thread::JoinHandle<()>) -> Self {
        Self { tx, worker_handle: Mutex::new(Some(worker_handle)) }
    }

    pub fn send(&self, task: WorkerTask<T>) {
        if let Err(e) = self.tx.send(task) {
            eprintln!("Failed to send a task: {:?}", e);
        }
    }

    pub fn terminate(&self) {
        // Send a shutdown signal to ensure the worker thread notices the shutdown
        self.send(WorkerTask::Finish);

        self.worker_handle.lock().unwrap().take().unwrap().join().unwrap();
    }
}
