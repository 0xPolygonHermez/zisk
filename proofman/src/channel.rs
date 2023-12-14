use std::sync::{Arc, RwLock};
use crossbeam_channel::{Sender, Receiver, unbounded};

// Naive implementation of a broadcast channel
// TODO: Implement a more efficient broadcast channel

// Struct representing the Broadcast object
pub struct SenderB<T> {
    // Internal channel for sending messages to the receiver
    sender: Sender<T>,

    // Internal receiver and queues for subscribers
    receivers: Arc<RwLock<Vec<(Sender<T>, Receiver<T>)>>>,
}

pub type ReceiverB<T> = Receiver<T>;

// Implementation of Broadcast methods
impl<T: Clone> SenderB<T> {
    // Constructor for creating a new Broadcast object
    pub fn new() -> Self {
        // Create an unbounded channel for communication between sender and receiver
        let (sender, receiver) = unbounded();

        // Wrap the receiver in an Arc<Mutex<>> for safe sharing among multiple subscribers
        let receivers = Arc::new(RwLock::new(vec![(sender.clone(), receiver)]));

        // Return a new Broadcast object with the sender and receiver
        SenderB { sender, receivers }
    }

    // Method for sending a message to all subscribers
    pub fn send(&self, message: T) {
        // Clone all senders and send the message to each one
        let receivers = self.receivers.read().unwrap().clone();
        for (sender, _) in receivers.iter() {
            sender.send(message.clone()).unwrap();
        }
    }

    // Method for subscribing to the Broadcast, returning a sender and a receiver
    pub fn subscribe(&self) -> ReceiverB<T> {
        // Create a new sender and receiver and add them to the list of subscribers
        let (sender, receiver) = unbounded();
        self.receivers.write().unwrap().push((sender.clone(), receiver.clone()));
        receiver
    }
}

impl<T> Clone for SenderB<T> {
    fn clone(&self) -> Self {
        SenderB {
            sender: self.sender.clone(),
            receivers: self.receivers.clone(),
        }
    }
}