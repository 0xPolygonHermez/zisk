use num_bigint::BigUint;
use num_traits::Num;

pub fn str_test_data<const N_IN: usize, const N_OUT: usize>(
    index: usize,
    title: &str,
    data: [&'static str; N_IN],
) -> Vec<[u64; N_OUT]> {
    data.iter()
        .flat_map(|x| {
            let mut _data = if let Some(stripped) = x.strip_prefix("0x") {
                BigUint::from_str_radix(stripped, 16)
            } else {
                BigUint::from_str_radix(x, 10)
            }
            .unwrap_or_else(|_| panic!("Failed to parse #{} {} string : '{}'", index, title, &x))
            .to_u64_digits();
            _data.resize(4, 0);
            _data
        })
        .collect::<Vec<_>>()
        .chunks_exact(N_OUT)
        .map(|chunk| {
            chunk
                .try_into()
                .unwrap_or_else(|_| panic!("Failed to split #{index} {title} in {N_OUT} elements"))
        })
        .collect::<Vec<[u64; N_OUT]>>()
}
