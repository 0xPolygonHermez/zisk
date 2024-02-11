use goldilocks::{Goldilocks, AbstractField};
use proofman::public_inputs::PublicInputs;

#[derive(Debug)]
pub struct ZKEVMPublicInputs {
    // oldStateRoot
    pub b0: Goldilocks,
    pub b1: Goldilocks,
    pub b2: Goldilocks,
    pub b3: Goldilocks,
    pub b4: Goldilocks,
    pub b5: Goldilocks,
    pub b6: Goldilocks,
    pub b7: Goldilocks,
    // oldAccInputHash
    pub c0: Goldilocks,
    pub c1: Goldilocks,
    pub c2: Goldilocks,
    pub c3: Goldilocks,
    pub c4: Goldilocks,
    pub c5: Goldilocks,
    pub c6: Goldilocks,
    pub c7: Goldilocks,
    // oldBatchNum
    pub sp: Goldilocks,
    // chainId
    pub gas: Goldilocks,
    // forkid
    pub ctx: Goldilocks,
    // newStateRoot
    pub sr0: Goldilocks,
    pub sr1: Goldilocks,
    pub sr2: Goldilocks,
    pub sr3: Goldilocks,
    pub sr4: Goldilocks,
    pub sr5: Goldilocks,
    pub sr6: Goldilocks,
    pub sr7: Goldilocks,
    // newAccInputHash
    pub d0: Goldilocks,
    pub d1: Goldilocks,
    pub d2: Goldilocks,
    pub d3: Goldilocks,
    pub d4: Goldilocks,
    pub d5: Goldilocks,
    pub d6: Goldilocks,
    pub d7: Goldilocks,
    // localExitRoot
    pub e0: Goldilocks,
    pub e1: Goldilocks,
    pub e2: Goldilocks,
    pub e3: Goldilocks,
    pub e4: Goldilocks,
    pub e5: Goldilocks,
    pub e6: Goldilocks,
    pub e7: Goldilocks,
    // newBatchNum
    pub pc: Goldilocks,
    // constRoot
    pub cr0: Goldilocks,
    pub cr1: Goldilocks,
    pub cr2: Goldilocks,
    pub cr3: Goldilocks,
}

impl PublicInputs<Goldilocks> for ZKEVMPublicInputs {
    fn to_vec(&self) -> Vec<Goldilocks> {
        vec![
            self.b0, self.b1, self.b2, self.b3, self.b4, self.b5, self.b6, self.b7, self.c0, self.c1, self.c2, self.c3,
            self.c4, self.c5, self.c6, self.c7, self.sp, self.gas, self.ctx, self.sr0, self.sr1, self.sr2, self.sr3,
            self.sr4, self.sr5, self.sr6, self.sr7, self.d0, self.d1, self.d2, self.d3, self.d4, self.d5, self.d6,
            self.d7, self.e0, self.e1, self.e2, self.e3, self.e4, self.e5, self.e6, self.e7, self.pc, self.cr0,
            self.cr1, self.cr2, self.cr3,
        ]
    }
}

impl Default for ZKEVMPublicInputs {
    fn default() -> Self {
        Self {
            b0: Goldilocks::zero(),
            b1: Goldilocks::zero(),
            b2: Goldilocks::zero(),
            b3: Goldilocks::zero(),
            b4: Goldilocks::zero(),
            b5: Goldilocks::zero(),
            b6: Goldilocks::zero(),
            b7: Goldilocks::zero(),
            c0: Goldilocks::zero(),
            c1: Goldilocks::zero(),
            c2: Goldilocks::zero(),
            c3: Goldilocks::zero(),
            c4: Goldilocks::zero(),
            c5: Goldilocks::zero(),
            c6: Goldilocks::zero(),
            c7: Goldilocks::zero(),
            sp: Goldilocks::zero(),
            gas: Goldilocks::zero(),
            ctx: Goldilocks::zero(),
            sr0: Goldilocks::zero(),
            sr1: Goldilocks::zero(),
            sr2: Goldilocks::zero(),
            sr3: Goldilocks::zero(),
            sr4: Goldilocks::zero(),
            sr5: Goldilocks::zero(),
            sr6: Goldilocks::zero(),
            sr7: Goldilocks::zero(),
            d0: Goldilocks::zero(),
            d1: Goldilocks::zero(),
            d2: Goldilocks::zero(),
            d3: Goldilocks::zero(),
            d4: Goldilocks::zero(),
            d5: Goldilocks::zero(),
            d6: Goldilocks::zero(),
            d7: Goldilocks::zero(),
            e0: Goldilocks::zero(),
            e1: Goldilocks::zero(),
            e2: Goldilocks::zero(),
            e3: Goldilocks::zero(),
            e4: Goldilocks::zero(),
            e5: Goldilocks::zero(),
            e6: Goldilocks::zero(),
            e7: Goldilocks::zero(),
            pc: Goldilocks::zero(),
            cr0: Goldilocks::zero(),
            cr1: Goldilocks::zero(),
            cr2: Goldilocks::zero(),
            cr3: Goldilocks::zero(),
        }
    }
}
