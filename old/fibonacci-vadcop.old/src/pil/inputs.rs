use std::{error::Error, fs::File, io::Read, path::PathBuf};

pub struct FibonacciVadcopInputs {
    pub a: usize,
    pub b: usize,
    pub module: usize
}

#[allow(unused)]
impl FibonacciVadcopInputs {
    pub fn from_json(inputs_path: Option<PathBuf>) -> Result<FibonacciVadcopInputs, Box<dyn Error>> {
        // Open the file and read the file content
        let path = inputs_path.expect("A valid path must be provided.");
        let mut file = File::open(path).expect("File not found.");

        let mut content = String::new();
        file.read_to_string(&mut content).expect("Failed to read file.");

        // Parse the JSON
        let values: Vec<usize> = serde_json::from_str(&content).expect("Failed to parse JSON.");

        // Ensure the JSON array has exactly three values
        assert_eq!(values.len(), 3, "JSON array must contain exactly three values.");

        // Return the inputs
        Ok(FibonacciVadcopInputs {
            a: values[0],
            b: values[1],
            module: values[2],
        })
    }

    pub fn from_bytes(input_bytes: &[u8]) -> FibonacciVadcopInputs {
        const USIZE_SIZE: usize = std::mem::size_of::<usize>();
        assert_eq!(input_bytes.len(), USIZE_SIZE * 3, "Input bytes length must be 3 * size_of::<usize>()");

        let a_bytes = input_bytes[0..USIZE_SIZE].try_into().expect("Slice with incorrect length for a");
        let b_bytes = input_bytes[USIZE_SIZE..2 * USIZE_SIZE].try_into().expect("Slice with incorrect length for b");
        let module_bytes = input_bytes[2 * USIZE_SIZE..3 * USIZE_SIZE].try_into().expect("Slice with incorrect length for module");

        let a = usize::from_le_bytes(a_bytes);
        let b = usize::from_le_bytes(b_bytes);
        let module = usize::from_le_bytes(module_bytes);

        FibonacciVadcopInputs { a, b, module }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.a.to_le_bytes());
        bytes.extend_from_slice(&self.b.to_le_bytes());
        bytes.extend_from_slice(&self.module.to_le_bytes());
        bytes
    }
}
