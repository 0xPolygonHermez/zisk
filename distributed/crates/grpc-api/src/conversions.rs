use crate::{
    ComputeCapacity as GrpcComputeCapacity,
    Proof as GrpcProof,
    // ProverAllocation as GrpcProverAllocation,
};
use distributed_common::{AggProofData, ComputeCapacity};

/// Conversions between coordinator-common types and gRPC types
/// This module handles the translation layer between our domain types
/// and the generated gRPC protobuf types.
impl From<ComputeCapacity> for GrpcComputeCapacity {
    fn from(capacity: ComputeCapacity) -> Self {
        GrpcComputeCapacity { compute_units: capacity.compute_units }
    }
}

impl From<GrpcComputeCapacity> for ComputeCapacity {
    fn from(grpc_capacity: GrpcComputeCapacity) -> Self {
        ComputeCapacity { compute_units: grpc_capacity.compute_units }
    }
}

impl From<AggProofData> for GrpcProof {
    fn from(row_data: AggProofData) -> Self {
        GrpcProof {
            airgroup_id: row_data.airgroup_id,
            values: row_data.values,
            worker_idx: row_data.worker_idx,
        }
    }
}

impl From<GrpcProof> for AggProofData {
    fn from(grpc_row_data: GrpcProof) -> Self {
        AggProofData {
            airgroup_id: grpc_row_data.airgroup_id,
            values: grpc_row_data.values,
            worker_idx: grpc_row_data.worker_idx,
        }
    }
}
