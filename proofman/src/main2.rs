use std::sync::{mpsc, Arc, Mutex};
use std::thread;

mod air;

use air::trace::trace_column::TraceColumn;
use air::trace::trace_layout::TraceLayout;
use air::trace::trace::{Trace, StoreType};
use air::mock_base_field::mock_base_field::{MockBaseField, BaseFieldType};

// Define a message type for communication
// #[derive(Debug)]
// enum Message {
//     Request(String),
//     Response(String),
// }

fn main() {
    // Create a full-duplex channel
    let (sender_a, receiver_b) = mpsc::channel::<Trace>();
    let (sender_b, receiver_a) = mpsc::channel::<Trace>();

    // Wrap channels in Arc and Mutex for shared ownership between threads
    let sender_a = Arc::new(Mutex::new(sender_a));
    let sender_b = Arc::new(Mutex::new(sender_b));

    // Spawn two threads
    let thread_a = spawn_thread_fibonacci("Thread Fibonacci", receiver_a, sender_b.clone());
    let thread_b = spawn_thread_module("Thread Module", receiver_b, sender_a.clone());

    // Wait for both threads to finish
    thread_a.join().unwrap();
    thread_b.join().unwrap();
}

// Function to spawn a thread
fn spawn_thread_fibonacci(thread_name: &'static str, _receiver: mpsc::Receiver<Trace>, sender: Arc<Mutex<mpsc::Sender<Trace>>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let trace = get_fibonacci_trace();
        sender.lock().unwrap().send(trace).unwrap();
        // sender.lock().unwrap().send(Message::Response(format!("{}: Trace {:?}", thread_name, trace))).unwrap();
    })
}

fn spawn_thread_module(thread_name: &'static str, receiver: mpsc::Receiver<Trace>, _sender: Arc<Mutex<mpsc::Sender<Trace>>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let response = receiver.recv().unwrap();
        println!("{}: Received {:?}", thread_name, response);
    })
}

fn get_fibonacci_trace() -> Trace {
    let num_rows = 2usize.pow(3);

    // Create Trace Layout
    let mut trace_layout = TraceLayout::new(num_rows);

    trace_layout.add_column(TraceColumn::new("witness.a", MockBaseField::SIZE));
    trace_layout.add_column(TraceColumn::new("witness.b", MockBaseField::SIZE));
    trace_layout.add_column(TraceColumn::new("fixed.L1", MockBaseField::SIZE));
    trace_layout.add_column(TraceColumn::new("fixed.LLAST", MockBaseField::SIZE));

    // Create Mock Data values for witness and fixed columns
    let mut witness_a = Vec::<MockBaseField>::new();
    let mut witness_b = Vec::<MockBaseField>::new();
    let mut fixed_l1 = Vec::<MockBaseField>::new();
    let mut fixed_llast = Vec::<MockBaseField>::new();

    let mut a = 1;
    let mut b = 1;

    for i in 0..num_rows {
        witness_a.push(MockBaseField::new(BaseFieldType::NoExtended, &[a]));
        witness_b.push(MockBaseField::new(BaseFieldType::NoExtended, &[b]));
        fixed_l1.push(MockBaseField::new(BaseFieldType::NoExtended, &[if i == 0 { 1 } else { 0 }]));
        fixed_llast.push(MockBaseField::new(BaseFieldType::NoExtended, &[if i == num_rows - 1 { 1 } else { 0 }]));

        let temp = a;
        a = b;
        b = temp + b;
    }

    // Create Trace
    let mut trace: Trace = Trace::new(trace_layout, StoreType::RowMajor);

    trace.set_column("witness.a", &witness_a);
    trace.set_column("witness.b", &witness_b);
    trace.set_column("fixed.L1", &fixed_l1);
    trace.set_column("fixed.LLAST", &fixed_llast);

    trace
    // println!("trace: {:?}", trace);

    // println!("col_a: {:?}", trace.get_column("witness.a"));
    // println!("col_b: {:?}", trace.get_column("witness.b"));
    // println!("col_l1: {:?}", trace.get_column("fixed.L1"));
    // println!("col_llast: {:?}", trace.get_column("fixed.LLAST"));
}