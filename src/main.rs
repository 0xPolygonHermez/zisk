mod air;
use air::trace::trace_column::TraceColumn;
use air::trace::trace_layout::TraceLayout;
use air::trace::trace::Trace;

fn main() {
    let trace_col_a =  TraceColumn::new(String::from("colA"), 8);
    let trace_col_b =  TraceColumn::new(String::from("colB"), 24);
    let trace_col_c =  TraceColumn::new(String::from("colC"), 8);

    let vec: Vec<TraceColumn> = Vec::new();
    let mut trace_layout = TraceLayout::new(vec, 8);
    trace_layout.add_column(trace_col_a);
    trace_layout.add_column(trace_col_b);
    trace_layout.add_column(trace_col_c);

    let trace = Trace::new(trace_layout, String::from("row_major"), String::from("mem"));

    println!("trace_layout: {:?}", trace);
}
