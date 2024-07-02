use std::path::PathBuf;

pub struct FibonacciVadcopInputs {
    pub a: usize,
    pub b: usize,
}

impl FibonacciVadcopInputs {
    pub fn to_bytes(_inputs_path: Option<PathBuf>) -> Vec<u8> {
        let inputs: Vec<usize> = vec![17, 1, 2];

        inputs.iter().flat_map(|input| input.to_le_bytes().to_vec()).collect()
    }

    pub fn from_bytes(input_bytes: &[u8]) -> Vec<usize> {
        let usize_size = std::mem::size_of::<usize>();
        assert_eq!(input_bytes.len() % usize_size, 0);

        let mut inputs = Vec::new();
        for chunk in input_bytes.chunks(usize_size) {
            let array: [u8; std::mem::size_of::<usize>()] = chunk.try_into().expect("Slice with incorrect length");
            inputs.push(usize::from_le_bytes(array));
        }

        inputs
    }
}
