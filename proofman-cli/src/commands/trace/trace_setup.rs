use clap::Args;
use proofman::command_handlers::trace_setup_handler::trace_setup_handler;

#[derive(Args)]
pub struct TraceSetupCmd {}

impl TraceSetupCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        trace_setup_handler(&"./examples/fibv/data/fibv.pilout".into(), &".".into())
    }
}
