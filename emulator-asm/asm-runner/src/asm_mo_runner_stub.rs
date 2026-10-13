//! Non-Linux-x86_64 stub: placeholder types whose methods error/panic. Off the
//! supported platform these are never exercised, so they carry no docs.
#![allow(missing_docs)]

use crate::OwnedMemInstances;
use zisk_common::{ExecutorStatsHandle, Plan};

use std::fmt::Debug;

use anyhow::Result;

pub struct MOShmemReader {}

// This struct is used to run the assembly code in a separate process and generate minimal traces.
#[derive(Debug)]
pub struct AsmRunnerMO {
    pub plans: Vec<Plan>,
    /// Bytes of the proofman-owned GPU buffer consumed by the GPU mem-ops
    /// planner; always `None` on this stub (no GPU planner on unsupported targets).
    pub gpu_mops_used_bytes: Option<u64>,
    pub device_witness: Option<DeviceMemWitness>,
}

impl AsmRunnerMO {
    pub fn new(plans: Vec<Plan>) -> Self {
        AsmRunnerMO { plans, gpu_mops_used_bytes: None, device_witness: None }
    }

    pub fn run(
        _: &mut MOShmemReader,
        _: u64,
        _: u64,
        _: i32,
        _: i32,
        _: Option<u16>,
        _: ExecutorStatsHandle,
    ) -> Result<Self> {
        Err(anyhow::anyhow!(
            "AsmRunnerMO::run() is not supported on this platform. Only Linux x86_64 is supported."
        ))
    }
}

/// The memory instances this process owns, by family (no device witness on this platform).
/// Never built on this platform.
#[derive(Debug)]
pub struct DeviceMemWitness;

impl DeviceMemWitness {
    pub fn fill_owned(
        &self,
        _owned: &OwnedMemInstances<'_>,
        _d_buffers: *mut std::ffi::c_void,
    ) -> bool {
        false
    }
}

/// No device witness on this platform.
pub fn device_mem_witness_end() {}

pub fn device_mem_witness_failed(_reason: &str) {}

/// No device witness on this platform.
pub fn device_mem_witness_requested() -> bool {
    zisk_common::device_mem_witness()
}

/// The light memory-ops emulator only on request (`ZISK_MOPS_LIGHT=1`).
pub fn mops_light() -> bool {
    match std::env::var("ZISK_MOPS_LIGHT").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        _ => !device_mem_witness_requested(),
    }
}
