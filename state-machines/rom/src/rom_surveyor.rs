use std::any::Any;

use sm_common::{SurveyStats, Surveyor};
use zisk_core::{InstContext, ZiskInst};

#[derive(Default)]
pub struct RomSurveyor {
    pub rom: SurveyStats,
}

impl Surveyor for RomSurveyor {
    fn survey(&mut self, _: &ZiskInst, inst_ctx: &InstContext) {
        self.rom.update(inst_ctx.pc, 1);
    }

    fn add(&mut self, other: &dyn Surveyor) {
        if let Some(other) = other.as_any().downcast_ref::<RomSurveyor>() {
            for (k, v) in &other.rom.inst_count {
                let count = self.rom.inst_count.entry(*k).or_default();
                *count += *v;
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for RomSurveyor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "RomSurveyor {{ rom entries: {:?} }}", self.rom.inst_count.len())
        // write dummy
        write!(f, "RomSurveyor {{ rom entries: {} }}", self.rom.inst_count.len())
    }
}
