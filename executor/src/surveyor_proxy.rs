use sm_common::Surveyor;
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst};

#[derive(Default)]
pub struct SurveyorProxy {
    pub surveyors: Vec<Box<dyn Surveyor>>,
}

impl SurveyorProxy {
    pub fn new() -> Self {
        Self { surveyors: Vec::new() }
    }

    pub fn register_surveyor(&mut self, observer: Box<dyn Surveyor>) {
        self.surveyors.push(observer);
    }
}

impl InstObserver for SurveyorProxy {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) {
        for observer in &mut self.surveyors {
            (*observer).survey(zisk_inst, inst_ctx);
        }
    }
}
