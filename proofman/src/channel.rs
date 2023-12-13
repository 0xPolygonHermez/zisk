use crossbeam_channel::{Receiver, Sender, TryRecvError, RecvError};
use log::trace;

use crate::message::{Message, Payload};

pub struct Channel {
    pub tx: Sender<Message>,
    pub rx: Receiver<Message>
}

impl Channel {
    pub fn new(tx: Sender<Message>, rx: Receiver<Message>) -> Self {
        Channel { tx, rx }
    }

    pub fn send(&self, src: String, payload: Payload) {
        let msg = Message { src, dst: "*".to_string(), payload };
        trace!("channel > Sending message: {:?}", msg);
        self.tx.send(msg).unwrap();
    }

    pub fn recv(&self) -> Result<Message, RecvError> {
        let msg = self.rx.recv();
        trace!("channel > Message received: {:?}", msg);
        msg
    }

    pub fn try_recv(&self) -> Result<Message, TryRecvError> {
        trace!("channel > Trying to receive message");
        self.rx.try_recv()
    }
}