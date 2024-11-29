use sm_common::{SurveyCounter, Surveyor};
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

#[derive(Default)]
pub struct BinarySurveyor {
    pub binary: SurveyCounter,
    pub binary_extension: SurveyCounter,
}

impl Surveyor for BinarySurveyor {
    fn survey(&mut self, inst: &ZiskInst, _: &InstContext) {
        match inst.op_type {
            ZiskOperationType::Binary => {
                self.binary.update(1);
            }
            ZiskOperationType::BinaryE => {
                self.binary_extension.update(1);
            }
            _ => {}
        }
    }

    fn add(&mut self, other: &dyn Surveyor) {
        if let Some(other) = other.as_any().downcast_ref::<BinarySurveyor>() {
            self.binary.update(other.binary.inst_count);
            self.binary_extension.update(other.binary_extension.inst_count);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl std::fmt::Debug for BinarySurveyor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BinarySurveyor {{ binary: {:?}, binary_extension: {:?} }}",
            self.binary, self.binary_extension
        )
    }
}
