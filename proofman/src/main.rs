// use std::sync::mpsc;
// use std::thread;
use std::mem;

mod air;

use air::trace::trace_layout::TraceLayout;
use air::trace::trace::{Trace, StoreType};

use math::FieldElement;
use math::fields::CubeExtension;
use math::fields::f64::BaseElement;
use math::StarkField;


fn print_info<B: StarkField>(element: B) {

    println!("value: {} ({:?} bytes)", element, mem::size_of_val(&element));
}

fn main() {
    // let order = 2_u128.pow(64) - 2_u128.pow(32) + 1;
    //let max = order as u64 - 1;

    let base_element= BaseElement::new(1);
    print_info(base_element);

    println!("{:?}", FieldElement::elements_as_bytes(&[base_element]));

    let ext_element = <CubeExtension<BaseElement>>::new(
        BaseElement::new(3),
        BaseElement::new(5),
        BaseElement::new(2),
    );
    println!("value: {} ({:?} bytes)", ext_element, mem::size_of_val(&ext_element));

    println!("{:?}", FieldElement::elements_as_bytes(&[ext_element]));

    let u8_vector = [1, 0, 0, 0, 0, 0, 0, 0];
    let element_3  = unsafe { BaseElement::bytes_as_elements(&u8_vector).unwrap() };

    println!("{:?}", element_3[0]);

    test_trace_air_context();

    // let c = GoldilocksField(max);
    // let bytes = c.0;
    // println!("bytes BE: {:?}", bytes.to_be_bytes());
    // println!("bytes LE: {:?}", bytes.to_le_bytes());

    // println!("size: {} bytes", mem::size_of::<GoldilocksField>());

    //let d: QuadraticExtension<GoldilocksField>;
    // println!("size: {} bytes", mem::size_of::<QuadraticExtension<GoldilocksField>>());
    // println!("size: {} bytes", mem::size_of::<QuarticExtension<GoldilocksField>>());
    // println!("size: {} bytes", mem::size_of::<QuinticExtension<GoldilocksField>>());






    // let (tx, _rx) = mpsc::channel();

    // let trace = get_fibonacci_trace();

    // let handle = thread::spawn(move || worker_function(tx, trace));

    // handle.join().unwrap();
}

fn test_trace_air_context() {
    // Check traceCtx is working
    let num_rows = 2usize.pow(4);

    assert!(num_rows >= 4);
    assert!(num_rows.is_power_of_two());

    // Create Trace Layout
    let mut trace_layout = TraceLayout::new(num_rows);

    trace_layout.add_column("witness.a".to_string(), mem::size_of::<BaseElement>());
    trace_layout.add_column("witness.b".to_string(), mem::size_of::<BaseElement>());
    trace_layout.add_column("fixed.L1".to_string(), mem::size_of::<CubeExtension<BaseElement>>());
    trace_layout.add_column("fixed.LLAST".to_string(), mem::size_of::<CubeExtension<BaseElement>>());

    let mut witness_a = vec![BaseElement::default(); num_rows];
    let mut witness_b = vec![BaseElement::default(); num_rows];
    let mut fixed_l1 = vec![<CubeExtension<BaseElement>>::default(); num_rows];
    let mut fixed_llast = vec![<CubeExtension<BaseElement>>::default(); num_rows];

    witness_a[0] = BaseElement::new(1);
    witness_b[0] = BaseElement::new(1);
    for i in 1..num_rows {
        let temp = witness_a[i - 1];
        witness_a[i] = witness_b[i - 1];
        witness_b[i] = temp + witness_b[i - 1];
    }
    fixed_l1[0] = CubeExtension::new(BaseElement::new(1), BaseElement::new(0), BaseElement::new(0));
    fixed_llast[num_rows - 1] = CubeExtension::new(BaseElement::new(1), BaseElement::new(0), BaseElement::new(0));

    let mut trace_air_context = Trace::new(&trace_layout, StoreType::RowMajor);
    trace_air_context.new_trace(num_rows);
    trace_air_context.set_column_u8("witness.a", witness_a.len(), FieldElement::elements_as_bytes(&witness_a));
    trace_air_context.set_column_u8("witness.b", witness_b.len(), FieldElement::elements_as_bytes(&witness_b));
    trace_air_context.set_column_u8("fixed.L1", fixed_l1.len(), FieldElement::elements_as_bytes(&fixed_l1));
    trace_air_context.set_column_u8("fixed.LLAST", fixed_llast.len(), FieldElement::elements_as_bytes(&fixed_llast));

    println!("trace_air_context_2: {:?}", trace_air_context);


    //ctx.add(subprood_id, air_id, trace);






}

// fn worker_function(_tx: mpsc::Sender<String>, trace: Trace) {
//     println!("Hello from worker_function!");
//     println!("trace: {:?}", trace);
//     println!("col_a: {:?}", trace.get_column("witness.a"));
//     println!("col_b: {:?}", trace.get_column("witness.b"));
//     println!("col_l1: {:?}", trace.get_column("fixed.L1"));
//     println!("col_llast: {:?}", trace.get_column("fixed.LLAST"));
// }

// fn get_fibonacci_trace() -> Trace {
//     let num_rows = 2usize.pow(3);

//     // Create Trace Layout
//     let mut trace_layout = TraceLayout::new(num_rows);

//     trace_layout.add_column("witness.a".to_string(), MockBaseField::SIZE as usize);
//     trace_layout.add_column("witness.b".to_string(), MockBaseField::SIZE as usize);
//     trace_layout.add_column("fixed.L1".to_string(), MockBaseField::SIZE as usize);
//     trace_layout.add_column("fixed.LLAST".to_string(), MockBaseField::SIZE as usize);

//     // Create Mock Data values for witness and fixed columns
//     let mut witness_a = Vec::<MockBaseField>::new();
//     let mut witness_b = Vec::<MockBaseField>::new();
//     let mut fixed_l1 = Vec::<MockBaseField>::new();
//     let mut fixed_llast = Vec::<MockBaseField>::new();

//     let mut a = 1;
//     let mut b = 1;

//     for i in 0..num_rows {
//         witness_a.push(MockBaseField::new(BaseFieldType::NoExtended, &[a]));
//         witness_b.push(MockBaseField::new(BaseFieldType::NoExtended, &[b]));
//         fixed_l1.push(MockBaseField::new(BaseFieldType::NoExtended, &[if i == 0 { 1 } else { 0 }]));
//         fixed_llast.push(MockBaseField::new(BaseFieldType::NoExtended, &[if i == num_rows - 1 { 1 } else { 0 }]));

//         let temp = a;
//         a = b;
//         b = temp + b;
//     }

//     // Create Trace
//     let mut trace: Trace = Trace::new(trace_layout, StoreType::RowMajor);

//     trace.set_column("witness.a", &witness_a);
//     trace.set_column("witness.b", &witness_b);
//     trace.set_column("fixed.L1", &fixed_l1);
//     trace.set_column("fixed.LLAST", &fixed_llast);

//     trace
// }

// // use std::sync::{mpsc, Arc, Mutex};
// // use std::thread;

// // struct Broadcaster<T> {
// //     senders: Vec<mpsc::Sender<T>>,
// // }

// // impl<T> Broadcaster<T> {
// //     fn new() -> Self {
// //         Broadcaster {
// //             senders: Vec::new(),
// //         }
// //     }

// //     fn add_receiver(&mut self) -> mpsc::Receiver<T> {
// //         let (tx, rx) = mpsc::channel();
// //         self.senders.push(tx);
// //         rx
// //     }

// //     fn broadcast(&self, msg: T)
// //     where
// //         T: Clone,
// //     {
// //         for sender in &self.senders {
// //             sender.send(msg.clone()).unwrap();
// //         }
// //     }
// // }

// // fn main() {
// //     let broadcaster = Arc::new(Mutex::new(Broadcaster::new()));

// //     // Create receivers
// //     let receiver1 = broadcaster.lock().unwrap().add_receiver();
// //     let receiver2 = broadcaster.lock().unwrap().add_receiver();

// //     // Spawn a thread to broadcast messages
// //     let broadcaster_clone = broadcaster.clone();
// //     thread::spawn(move || {
// //         for i in 0..5 {
// //             broadcaster_clone.lock().unwrap().broadcast(i);
// //             thread::sleep(std::time::Duration::from_secs(1));
// //         }
// //     });

// //     // Receive and print messages from the receivers
// //     for received in receiver1.iter().take(5) {
// //         println!("Receiver 1: {}", received);
// //     }

// //     for received in receiver2.iter().take(5) {
// //         println!("Receiver 2: {}", received);
// //     }
// // }
