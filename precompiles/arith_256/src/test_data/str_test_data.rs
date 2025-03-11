use num_bigint::BigUint;
use num_traits::Num;

pub fn str_test_data<const N_IN: usize, const N_OUT: usize>(
    index: usize,
    title: &str,
    data: [&'static str; N_IN],
) -> Vec<[u64; N_OUT]> {
    data.iter()
        .map(|x| {
            let mut _data = if x.starts_with("0x") {
                BigUint::from_str_radix(&x[2..], 16)
            } else {
                BigUint::from_str_radix(&x, 10)
            }
            .expect(&format!("Failed to parse #{} {} string : '{}'", index, title, &x))
            .to_u64_digits();
            _data.resize(4, 0);
            _data
        })
        .flat_map(|arr| arr)
        .collect::<Vec<_>>()
        .chunks_exact(N_OUT)
        .map(|chunk| {
            chunk
                .try_into()
                .expect(&format!("Failed to split #{} {} in {} elements", index, title, N_OUT))
        })
        .collect::<Vec<[u64; N_OUT]>>()
}
