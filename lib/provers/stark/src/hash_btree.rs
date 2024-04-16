use goldilocks::Field;

fn hash_btree<T>(mut input_array: Vec<T>) -> Result<T, &'static str>
where
    T: Default + Field,
{
    if input_array.is_empty() {
        return Err("Cannot hash an empty array");
    }

    if input_array.len() == 1 {
        return Ok(input_array.pop().unwrap());
    }

    // Pad the input array if it has odd length
    if input_array.len() % 2 != 0 {
        input_array.push(T::zero());
    }

    let mut result = Vec::with_capacity(input_array.len() / 2);

    // Process pairs of elements
    for chunk in input_array.chunks_exact(2) {
        let (left, right) = (chunk[0], chunk[1]);

        let is_left_zero = left.is_zero();
        let is_right_zero = right.is_zero();

        let hash = if is_left_zero && is_right_zero {
            T::zero()
        } else if is_left_zero {
            right
        } else if is_right_zero {
            left
        } else {
            left + right
        };
        result.push(hash);
    }

    // Recursively call the function with the new array of hashes
    hash_btree(result)
}

#[cfg(test)]
mod tests {
    use goldilocks::{AbstractField, Goldilocks};

    use super::*;

    fn into_goldilocks_vec(vec: Vec<u32>) -> Vec<Goldilocks> {
        vec.into_iter().map(|x| Goldilocks::from_canonical_u32(x)).collect()
    }

    #[test]
    fn test_hash_btree() {
        let elements_0: Vec<Goldilocks> = Vec::new();
        let hash = hash_btree(elements_0);

        assert_eq!(hash, Err("Cannot hash an empty array"));

        // Create test data
        let hash = hash_btree(into_goldilocks_vec(vec![0]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(0)));

        let hash = hash_btree(into_goldilocks_vec(vec![11]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(11)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(123)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111, 1111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(1234)));

        let hash = hash_btree(into_goldilocks_vec(vec![0, 0, 111, 1111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(1222)));

        let hash = hash_btree(into_goldilocks_vec(vec![0, 11, 111, 1111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(1233)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 0, 111, 1111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(1223)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 0, 1111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(1123)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111, 0]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(123)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(123)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(123)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111, 1111, 11111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(12345)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111, 1111, 11111, 111111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(123456)));

        let hash = hash_btree(into_goldilocks_vec(vec![1, 11, 111, 1111, 11111, 111111, 1111111]));
        assert_eq!(hash, Ok(Goldilocks::from_canonical_u32(1234567)));
    }
}
