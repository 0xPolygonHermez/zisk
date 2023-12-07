#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PilOut {
    #[prost(string, optional, tag = "1")]
    pub name: ::core::option::Option<::prost::alloc::string::String>,
    /// base field characteristic
    #[prost(bytes = "vec", tag = "2")]
    pub base_field: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag = "3")]
    pub subproofs: ::prost::alloc::vec::Vec<Subproof>,
    /// number of challenges per stage
    #[prost(uint32, repeated, tag = "4")]
    pub num_challenges: ::prost::alloc::vec::Vec<u32>,
    #[prost(uint32, tag = "5")]
    pub num_proof_values: u32,
    #[prost(uint32, tag = "6")]
    pub num_public_values: u32,
    #[prost(message, repeated, tag = "7")]
    pub public_tables: ::prost::alloc::vec::Vec<PublicTable>,
    #[prost(message, repeated, tag = "8")]
    pub expressions: ::prost::alloc::vec::Vec<GlobalExpression>,
    /// Constraints that apply only to signals
    #[prost(message, repeated, tag = "9")]
    pub constraints: ::prost::alloc::vec::Vec<GlobalConstraint>,
    #[prost(message, repeated, tag = "10")]
    pub hints: ::prost::alloc::vec::Vec<Hint>,
    #[prost(message, repeated, tag = "11")]
    pub symbols: ::prost::alloc::vec::Vec<Symbol>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Subproof {
    #[prost(string, optional, tag = "1")]
    pub name: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(bool, tag = "2")]
    pub aggregable: bool,
    #[prost(message, repeated, tag = "3")]
    pub subproofvalues: ::prost::alloc::vec::Vec<SubproofValue>,
    #[prost(message, repeated, tag = "4")]
    pub airs: ::prost::alloc::vec::Vec<BasicAir>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubproofValue {
    #[prost(enumeration = "AggregationType", tag = "1")]
    pub agg_type: i32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublicTable {
    #[prost(uint32, tag = "1")]
    pub num_cols: u32,
    #[prost(uint32, tag = "2")]
    pub max_rows: u32,
    #[prost(enumeration = "AggregationType", tag = "3")]
    pub agg_type: i32,
    #[prost(message, optional, tag = "4")]
    pub row_expression_idx: ::core::option::Option<global_operand::Expression>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GlobalConstraint {
    #[prost(message, optional, tag = "1")]
    pub expression_idx: ::core::option::Option<global_operand::Expression>,
    #[prost(string, optional, tag = "2")]
    pub debug_line: ::core::option::Option<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GlobalExpression {
    #[prost(oneof = "global_expression::Operation", tags = "1, 2, 3, 4")]
    pub operation: ::core::option::Option<global_expression::Operation>,
}
/// Nested message and enum types in `GlobalExpression`.
pub mod global_expression {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Add {
        #[prost(message, optional, tag = "1")]
        pub lhs: ::core::option::Option<super::GlobalOperand>,
        #[prost(message, optional, tag = "2")]
        pub rhs: ::core::option::Option<super::GlobalOperand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Sub {
        #[prost(message, optional, tag = "1")]
        pub lhs: ::core::option::Option<super::GlobalOperand>,
        #[prost(message, optional, tag = "2")]
        pub rhs: ::core::option::Option<super::GlobalOperand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Mul {
        #[prost(message, optional, tag = "1")]
        pub lhs: ::core::option::Option<super::GlobalOperand>,
        #[prost(message, optional, tag = "2")]
        pub rhs: ::core::option::Option<super::GlobalOperand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Neg {
        #[prost(message, optional, tag = "1")]
        pub value: ::core::option::Option<super::GlobalOperand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Operation {
        #[prost(message, tag = "1")]
        Add(Add),
        #[prost(message, tag = "2")]
        Sub(Sub),
        #[prost(message, tag = "3")]
        Mul(Mul),
        #[prost(message, tag = "4")]
        Neg(Neg),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GlobalOperand {
    #[prost(oneof = "global_operand::Operand", tags = "1, 2, 3, 4, 5, 6, 7, 8")]
    pub operand: ::core::option::Option<global_operand::Operand>,
}
/// Nested message and enum types in `GlobalOperand`.
pub mod global_operand {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Constant {
        /// basefield element, variable length
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Challenge {
        #[prost(uint32, tag = "1")]
        pub stage: u32,
        /// index relative to the stage
        #[prost(uint32, tag = "2")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ProofValue {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SubproofValue {
        #[prost(uint32, tag = "1")]
        pub subproof_id: u32,
        #[prost(uint32, tag = "2")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicValue {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicTableAggregatedValue {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicTableColumn {
        /// public table index
        #[prost(uint32, tag = "1")]
        pub idx: u32,
        /// column index within the table
        #[prost(uint32, tag = "2")]
        pub col_idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Expression {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Operand {
        #[prost(message, tag = "1")]
        Constant(Constant),
        #[prost(message, tag = "2")]
        Challenge(Challenge),
        #[prost(message, tag = "3")]
        ProofValue(ProofValue),
        #[prost(message, tag = "4")]
        SubproofValue(SubproofValue),
        #[prost(message, tag = "5")]
        PublicValue(PublicValue),
        #[prost(message, tag = "6")]
        PublicTableAggregatedValue(PublicTableAggregatedValue),
        #[prost(message, tag = "7")]
        PublicTableColumn(PublicTableColumn),
        #[prost(message, tag = "8")]
        Expression(Expression),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BasicAir {
    #[prost(string, optional, tag = "1")]
    pub name: ::core::option::Option<::prost::alloc::string::String>,
    /// log2(n), where n is the number of rows
    #[prost(uint32, optional, tag = "2")]
    pub num_rows: ::core::option::Option<u32>,
    #[prost(message, repeated, tag = "3")]
    pub periodic_cols: ::prost::alloc::vec::Vec<PeriodicCol>,
    #[prost(message, repeated, tag = "4")]
    pub fixed_cols: ::prost::alloc::vec::Vec<FixedCol>,
    /// stage widths excluding stage 0 (fixed columns)
    #[prost(uint32, repeated, tag = "5")]
    pub stage_widths: ::prost::alloc::vec::Vec<u32>,
    #[prost(message, repeated, tag = "6")]
    pub expressions: ::prost::alloc::vec::Vec<Expression>,
    #[prost(message, repeated, tag = "7")]
    pub constraints: ::prost::alloc::vec::Vec<Constraint>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeriodicCol {
    /// BaseFieldElement, only the cycle
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub values: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FixedCol {
    /// BaseFieldElement
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub values: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Constraint {
    #[prost(oneof = "constraint::Constraint", tags = "1, 2, 3, 4")]
    pub constraint: ::core::option::Option<constraint::Constraint>,
}
/// Nested message and enum types in `Constraint`.
pub mod constraint {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct FirstRow {
        #[prost(message, optional, tag = "1")]
        pub expression_idx: ::core::option::Option<super::operand::Expression>,
        #[prost(string, optional, tag = "2")]
        pub debug_line: ::core::option::Option<::prost::alloc::string::String>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct LastRow {
        #[prost(message, optional, tag = "1")]
        pub expression_idx: ::core::option::Option<super::operand::Expression>,
        #[prost(string, optional, tag = "2")]
        pub debug_line: ::core::option::Option<::prost::alloc::string::String>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct EveryRow {
        #[prost(message, optional, tag = "1")]
        pub expression_idx: ::core::option::Option<super::operand::Expression>,
        #[prost(string, optional, tag = "2")]
        pub debug_line: ::core::option::Option<::prost::alloc::string::String>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct EveryFrame {
        #[prost(message, optional, tag = "1")]
        pub expression_idx: ::core::option::Option<super::operand::Expression>,
        /// offsetMin = 0 means that current row is at index 0
        #[prost(uint32, tag = "2")]
        pub offset_min: u32,
        /// frame size is defined as offsetMax - offsetMin + 1
        #[prost(uint32, tag = "3")]
        pub offset_max: u32,
        #[prost(string, optional, tag = "4")]
        pub debug_line: ::core::option::Option<::prost::alloc::string::String>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Constraint {
        #[prost(message, tag = "1")]
        FirstRow(FirstRow),
        #[prost(message, tag = "2")]
        LastRow(LastRow),
        #[prost(message, tag = "3")]
        EveryRow(EveryRow),
        #[prost(message, tag = "4")]
        EveryFrame(EveryFrame),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Operand {
    #[prost(oneof = "operand::Operand", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
    pub operand: ::core::option::Option<operand::Operand>,
}
/// Nested message and enum types in `Operand`.
pub mod operand {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Constant {
        /// BaseFieldElement
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Challenge {
        #[prost(uint32, tag = "1")]
        pub stage: u32,
        /// index relative to the stage
        #[prost(uint32, tag = "2")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ProofValue {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SubproofValue {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicValue {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PeriodicCol {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
        #[prost(sint32, tag = "2")]
        pub row_offset: i32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct FixedCol {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
        #[prost(sint32, tag = "2")]
        pub row_offset: i32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct WitnessCol {
        #[prost(uint32, tag = "1")]
        pub stage: u32,
        /// index relative to the stage
        #[prost(uint32, tag = "2")]
        pub col_idx: u32,
        #[prost(sint32, tag = "3")]
        pub row_offset: i32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Expression {
        #[prost(uint32, tag = "1")]
        pub idx: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Operand {
        #[prost(message, tag = "1")]
        Constant(Constant),
        #[prost(message, tag = "2")]
        Challenge(Challenge),
        #[prost(message, tag = "3")]
        ProofValue(ProofValue),
        #[prost(message, tag = "4")]
        SubproofValue(SubproofValue),
        #[prost(message, tag = "5")]
        PublicValue(PublicValue),
        #[prost(message, tag = "6")]
        PeriodicCol(PeriodicCol),
        #[prost(message, tag = "7")]
        FixedCol(FixedCol),
        #[prost(message, tag = "8")]
        WitnessCol(WitnessCol),
        #[prost(message, tag = "9")]
        Expression(Expression),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Expression {
    #[prost(oneof = "expression::Operation", tags = "1, 2, 3, 4")]
    pub operation: ::core::option::Option<expression::Operation>,
}
/// Nested message and enum types in `Expression`.
pub mod expression {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Add {
        #[prost(message, optional, tag = "1")]
        pub lhs: ::core::option::Option<super::Operand>,
        #[prost(message, optional, tag = "2")]
        pub rhs: ::core::option::Option<super::Operand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Sub {
        #[prost(message, optional, tag = "1")]
        pub lhs: ::core::option::Option<super::Operand>,
        #[prost(message, optional, tag = "2")]
        pub rhs: ::core::option::Option<super::Operand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Mul {
        #[prost(message, optional, tag = "1")]
        pub lhs: ::core::option::Option<super::Operand>,
        #[prost(message, optional, tag = "2")]
        pub rhs: ::core::option::Option<super::Operand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Neg {
        #[prost(message, optional, tag = "1")]
        pub value: ::core::option::Option<super::Operand>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Operation {
        #[prost(message, tag = "1")]
        Add(Add),
        #[prost(message, tag = "2")]
        Sub(Sub),
        #[prost(message, tag = "3")]
        Mul(Mul),
        #[prost(message, tag = "4")]
        Neg(Neg),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Symbol {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(uint32, optional, tag = "2")]
    pub subproof_id: ::core::option::Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    pub air_id: ::core::option::Option<u32>,
    #[prost(enumeration = "SymbolType", tag = "4")]
    pub r#type: i32,
    #[prost(uint32, tag = "5")]
    pub id: u32,
    #[prost(uint32, optional, tag = "6")]
    pub stage: ::core::option::Option<u32>,
    #[prost(uint32, tag = "7")]
    pub dim: u32,
    #[prost(uint32, repeated, tag = "8")]
    pub lengths: ::prost::alloc::vec::Vec<u32>,
    #[prost(string, optional, tag = "9")]
    pub debug_line: ::core::option::Option<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HintField {
    #[prost(string, optional, tag = "1")]
    pub name: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(oneof = "hint_field::Value", tags = "2, 3, 4")]
    pub value: ::core::option::Option<hint_field::Value>,
}
/// Nested message and enum types in `HintField`.
pub mod hint_field {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(string, tag = "2")]
        StringValue(::prost::alloc::string::String),
        #[prost(message, tag = "3")]
        Operand(super::Operand),
        #[prost(message, tag = "4")]
        HintFieldArray(super::HintFieldArray),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HintFieldArray {
    #[prost(message, repeated, tag = "1")]
    pub hint_fields: ::prost::alloc::vec::Vec<HintField>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Hint {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "2")]
    pub hint_fields: ::prost::alloc::vec::Vec<HintField>,
    #[prost(uint32, optional, tag = "3")]
    pub subproof_id: ::core::option::Option<u32>,
    #[prost(uint32, optional, tag = "4")]
    pub air_id: ::core::option::Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum AggregationType {
    Sum = 0,
    Prod = 1,
}
impl AggregationType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            AggregationType::Sum => "SUM",
            AggregationType::Prod => "PROD",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SUM" => Some(Self::Sum),
            "PROD" => Some(Self::Prod),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SymbolType {
    ImCol = 0,
    FixedCol = 1,
    PeriodicCol = 2,
    WitnessCol = 3,
    ProofValue = 4,
    SubproofValue = 5,
    PublicValue = 6,
    PublicTable = 7,
    Challenge = 8,
}
impl SymbolType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            SymbolType::ImCol => "IM_COL",
            SymbolType::FixedCol => "FIXED_COL",
            SymbolType::PeriodicCol => "PERIODIC_COL",
            SymbolType::WitnessCol => "WITNESS_COL",
            SymbolType::ProofValue => "PROOF_VALUE",
            SymbolType::SubproofValue => "SUBPROOF_VALUE",
            SymbolType::PublicValue => "PUBLIC_VALUE",
            SymbolType::PublicTable => "PUBLIC_TABLE",
            SymbolType::Challenge => "CHALLENGE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "IM_COL" => Some(Self::ImCol),
            "FIXED_COL" => Some(Self::FixedCol),
            "PERIODIC_COL" => Some(Self::PeriodicCol),
            "WITNESS_COL" => Some(Self::WitnessCol),
            "PROOF_VALUE" => Some(Self::ProofValue),
            "SUBPROOF_VALUE" => Some(Self::SubproofValue),
            "PUBLIC_VALUE" => Some(Self::PublicValue),
            "PUBLIC_TABLE" => Some(Self::PublicTable),
            "CHALLENGE" => Some(Self::Challenge),
            _ => None,
        }
    }
}
