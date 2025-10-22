/// Strongly-typed proof formats
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofFormat {
    /// Raw, unprocessed proof data
    Raw(RawProof),
    /// Compressed proof using compression algorithms
    Compressed(CompressedProof),
    /// Wrapped proof with additional metadata
    Wrapped(WrappedProof),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    pub id: Option<String>,
    pub proof: Option<Vec<u64>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProof(pub Proof);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedProof {
    pub data: Proof,
    pub compression_info: CompressionInfo,
}

/// Information about compression applied to a proof
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionInfo {
    pub algorithm: String,
    pub level: u32,
    pub original_size: usize,
    pub compressed_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedProof {
    pub data: Proof,
    pub metadata: ProofMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofMetadata {
    pub created_at: std::time::SystemTime,
    pub version: String,
    pub additional_data: std::collections::HashMap<String, String>,
}





use anyhow::{Result, Context};

// ...existing code...

// Utility functions for converting between bytes and u64 vectors
fn bytes_to_u64_vec(bytes: &[u8]) -> Vec<u64> {
    bytes.chunks(8)
        .map(|chunk| {
            let mut array = [0u8; 8];
            for (i, &byte) in chunk.iter().enumerate() {
                if i < 8 {
                    array[i] = byte;
                }
            }
            u64::from_le_bytes(array)
        })
        .collect()
}

fn u64_vec_to_bytes(data: &[u64]) -> Vec<u8> {
    data.iter()
        .flat_map(|&num| num.to_le_bytes())
        .collect()
}

impl CompressedProof {
    /// Create a compressed proof from raw proof with default compression level (3)
    pub fn from_raw(raw: RawProof) -> Result<Self> {
        Self::from_raw_with_level(raw, 3)
    }

    /// Create a compressed proof from raw proof with specified compression level (1-22)
    pub fn from_raw_with_level(raw: RawProof, level: i32) -> Result<Self> {
        // Serialize the proof to bytes
        let original_data = bincode::serialize(&raw.0)
            .context("Failed to serialize proof for compression")?;
        
        let original_size = original_data.len();
        
        // Compress using zstd
        let compressed_data = zstd::encode_all(original_data.as_slice(), level)
            .context("Failed to compress proof with zstd")?;
        
        let compressed_size = compressed_data.len();
        
        // Create a new proof with compressed data
        let compressed_proof = Proof {
            id: raw.0.id.clone(),
            proof: Some(bytes_to_u64_vec(&compressed_data)),
        };
        
        let compression_info = CompressionInfo {
            algorithm: "zstd".to_string(),
            level: level as u32,
            original_size,
            compressed_size,
        };
        
        Ok(CompressedProof {
            data: compressed_proof,
            compression_info,
        })
    }

    /// Get compression ratio (original_size / compressed_size)
    pub fn compression_ratio(&self) -> f64 {
        if self.compression_info.compressed_size == 0 {
            return 1.0;
        }
        self.compression_info.original_size as f64 / self.compression_info.compressed_size as f64
    }
    
    /// Get space saved as percentage
    pub fn space_saved_percent(&self) -> f64 {
        if self.compression_info.original_size == 0 {
            return 0.0;
        }
        let saved = self.compression_info.original_size.saturating_sub(self.compression_info.compressed_size);
        (saved as f64 / self.compression_info.original_size as f64) * 100.0
    }
}

impl RawProof {
    /// Decompress from a compressed proof
    pub fn from_compressed(compressed: CompressedProof) -> Result<Self> {
        if compressed.compression_info.algorithm != "zstd" {
            return Err(anyhow::anyhow!(
                "Unsupported compression algorithm: {}",
                compressed.compression_info.algorithm
            ));
        }
        
        // Extract compressed bytes from the proof
        let compressed_bytes = match &compressed.data.proof {
            Some(data) => u64_vec_to_bytes(data),
            None => return Err(anyhow::anyhow!("No proof data found")),
        };
        
        // Decompress using zstd
        let decompressed_data = zstd::decode_all(compressed_bytes.as_slice())
            .context("Failed to decompress proof with zstd")?;
        
        // Deserialize back to proof
        let original_proof: Proof = bincode::deserialize(&decompressed_data)
            .context("Failed to deserialize decompressed proof")?;
        
        Ok(RawProof(original_proof))
    }
}

// From trait implementations for basic conversions
impl From<Proof> for RawProof {
    fn from(proof: Proof) -> Self {
        RawProof(proof)
    }
}

impl From<RawProof> for ProofFormat {
    fn from(raw: RawProof) -> Self {
        ProofFormat::Raw(raw)
    }
}

impl From<CompressedProof> for ProofFormat {
    fn from(compressed: CompressedProof) -> Self {
        ProofFormat::Compressed(compressed)
    }
}

impl From<WrappedProof> for ProofFormat {
    fn from(wrapped: WrappedProof) -> Self {
        ProofFormat::Wrapped(wrapped)
    }
}

// TryFrom for fallible conversions (compression/decompression)
impl TryFrom<RawProof> for CompressedProof {
    type Error = anyhow::Error;
    
    fn try_from(raw: RawProof) -> Result<Self> {
        CompressedProof::from_raw(raw)
    }
}

impl TryFrom<CompressedProof> for RawProof {
    type Error = anyhow::Error;
    
    fn try_from(compressed: CompressedProof) -> Result<Self> {
        RawProof::from_compressed(compressed)
    }
}

// Additional From implementations for convenience
impl From<&RawProof> for Result<CompressedProof> {
    fn from(raw: &RawProof) -> Self {
        CompressedProof::from_raw(raw.clone())
    }
}

impl From<&CompressedProof> for Result<RawProof> {
    fn from(compressed: &CompressedProof) -> Self {
        RawProof::from_compressed(compressed.clone())
    }
}

// ProofFormat convenience methods
impl ProofFormat {
    /// Extract the underlying proof data regardless of format
    pub fn proof(&self) -> &Proof {
        match self {
            ProofFormat::Raw(raw) => &raw.0,
            ProofFormat::Compressed(compressed) => &compressed.data,
            ProofFormat::Wrapped(wrapped) => &wrapped.data,
        }
    }

    /// Check if this is a raw proof
    pub fn is_raw(&self) -> bool {
        matches!(self, ProofFormat::Raw(_))
    }

    /// Check if this is a compressed proof
    pub fn is_compressed(&self) -> bool {
        matches!(self, ProofFormat::Compressed(_))
    }

    /// Check if this is a wrapped proof
    pub fn is_wrapped(&self) -> bool {
        matches!(self, ProofFormat::Wrapped(_))
    }

    /// Try to compress this proof format
    pub fn try_compress(self) -> Result<ProofFormat> {
        match self {
            ProofFormat::Raw(raw) => Ok(ProofFormat::Compressed(raw.try_into()?)),
            ProofFormat::Compressed(_) => Ok(self), // Already compressed
            ProofFormat::Wrapped(wrapped) => {
                let raw = RawProof(wrapped.data);
                Ok(ProofFormat::Compressed(raw.try_into()?))
            }
        }
    }
    
    /// Try to decompress this proof format
    pub fn try_decompress(self) -> Result<ProofFormat> {
        match self {
            ProofFormat::Raw(_) => Ok(self), // Already raw
            ProofFormat::Compressed(compressed) => Ok(ProofFormat::Raw(compressed.try_into()?)),
            ProofFormat::Wrapped(_) => Ok(self), // Keep as wrapped
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_with_from() -> Result<()> {
        let original_proof = Proof {
            id: Some("test_proof".to_string()),
            proof: Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        };
        
        let raw_proof = RawProof(original_proof.clone());
        
        // Using From trait
        let compressed: CompressedProof = raw_proof.try_into()?;
        
        // Check compression info
        assert_eq!(compressed.compression_info.algorithm, "zstd");
        assert!(compressed.compression_info.original_size > 0);
        
        // Using From trait to decompress
        let decompressed: RawProof = compressed.try_into()?;
        
        // Should match original
        assert_eq!(decompressed.0, original_proof);
        
        Ok(())
    }
    
    #[test]
    fn test_compression_methods() -> Result<()> {
        let proof = Proof {
            id: Some("test".to_string()),
            proof: Some(vec![42; 100]), // Larger data for better compression
        };
        
        let raw = RawProof(proof);
        
        // Test different compression levels
        let compressed_default = CompressedProof::from_raw(raw.clone())?;
        let compressed_high = CompressedProof::from_raw_with_level(raw.clone(), 9)?;
        
        // Higher compression should result in smaller size (usually)
        assert!(compressed_high.compression_info.level > compressed_default.compression_info.level);
        
        // Test compression ratio
        assert!(compressed_default.compression_ratio() > 1.0);
        assert!(compressed_default.space_saved_percent() > 0.0);
        
        Ok(())
    }
}