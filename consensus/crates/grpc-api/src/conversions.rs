use crate::{
    ComputeCapacity as GrpcComputeCapacity, Proof as GrpcProof,
    ProverAllocation as GrpcProverAllocation,
};
use consensus_common::{AggProofData, ComputeCapacity, ProverAllocationDto};

/// Conversions between consensus-common types and gRPC types
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
        GrpcProof { airgroup_id: row_data.airgroup_id, values: row_data.values }
    }
}

impl From<GrpcProof> for AggProofData {
    fn from(grpc_row_data: GrpcProof) -> Self {
        AggProofData { airgroup_id: grpc_row_data.airgroup_id, values: grpc_row_data.values }
    }
}

impl From<GrpcProverAllocation> for ProverAllocationDto {
    fn from(pb: GrpcProverAllocation) -> Self {
        Self { range: pb.range_start..pb.range_end }
    }
}

impl From<ProverAllocationDto> for GrpcProverAllocation {
    fn from(dto: ProverAllocationDto) -> Self {
        Self { range_start: dto.range.start, range_end: dto.range.end }
    }
}
