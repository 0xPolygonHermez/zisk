use crate::{
    execute_task_response::ResultData, ComputeCapacity as GrpcComputeCapacity, ProverAllocation as GrpcProverAllocation, Proof as GrpcProof
};
use consensus_common::{ComputeCapacity, ProverAllocationDto, RowData};

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

impl From<RowData> for GrpcProof {
    fn from(row_data: RowData) -> Self {
        GrpcProof { airgroup_id: row_data.airgroup_id, values: row_data.values }
    }
}

impl From<GrpcProof> for RowData {
    fn from(grpc_row_data: GrpcProof) -> Self {
        RowData { airgroup_id: grpc_row_data.airgroup_id, values: grpc_row_data.values }
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

// impl From<ResultData> for RowData {
//     fn from(result_data: ResultData) -> Self {
//         match result_data {
//             ResultData::Proofs(proofs) => Self {
//                 airgroup_id: proofs.airgroup_id,
//                 values: proofs.values,
//             },
//             ResultData::Challenges(challenges) => Self {
//                 airgroup_id: 0, // or some appropriate default
//                 values: challenges.values,
//             },
//         }
//     }
// }
