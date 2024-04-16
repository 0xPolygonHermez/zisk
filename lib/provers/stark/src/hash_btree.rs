use goldilocks::Field;

fn hash_btree<T>(input_array: &mut Vec<T>) -> Result<T, &'static str>
where
    T: Default + Clone + Field,
{
    if input_array.is_empty() {
        return Err("Cannot hash an empty array");
    }

    if input_array.len() == 1 {
        if input_array[0].is_zero() {
            return Err("All elements in the array are zero");
        }
        return Ok(input_array[0].clone());
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
    hash_btree(&mut result)
}

#[cfg(test)]
mod tests {
    use goldilocks::{AbstractField, Goldilocks};

    use super::*;

    fn hash_siblings<T>(left: T, right: T) -> T
    where
        T: Field,
    {
        left + right
    }

    #[test]
    fn test_hash_btree() {
        let leaf_void = Goldilocks::zero();
        let leaf_0 = Goldilocks::from_canonical_u32(1);
        let leaf_1 = Goldilocks::from_canonical_u32(11);
        let leaf_2 = Goldilocks::from_canonical_u32(111);
        let leaf_3 = Goldilocks::from_canonical_u32(1111);
        let leaf_4 = Goldilocks::from_canonical_u32(11111);
        let leaf_5 = Goldilocks::from_canonical_u32(111111);
        let leaf_6 = Goldilocks::from_canonical_u32(1111111);
        let leaf_7 = Goldilocks::from_canonical_u32(11111111);
        let leaf_8 = hash_siblings(leaf_0, leaf_1);
        let leaf_9 = hash_siblings(leaf_2, leaf_3);
        let leaf_10 = hash_siblings(leaf_4, leaf_5);
        let leaf_11 = hash_siblings(leaf_6, leaf_7);
        let leaf_12 = hash_siblings(leaf_8, leaf_9);
        let leaf_13 = hash_siblings(leaf_10, leaf_11);
        let leaf_14 = hash_siblings(leaf_12, leaf_13);

        let hash = hash_btree(&mut Vec::<Goldilocks>::new());
        assert_eq!(hash, Err("Cannot hash an empty array"));

        // Create test data
        let hash = hash_btree(&mut vec![leaf_void]);
        assert_eq!(hash, Err("All elements in the array are zero"));

        let hash = hash_btree(&mut vec![leaf_void, leaf_void]);
        assert_eq!(hash, Err("All elements in the array are zero"));

        let hash = hash_btree(&mut vec![leaf_void, leaf_void, leaf_void]);
        assert_eq!(hash, Err("All elements in the array are zero"));

        // Create test data
        let hash = hash_btree(&mut vec![leaf_0]);
        assert_eq!(hash, Ok(leaf_0));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1]);
        assert_eq!(hash, Ok(leaf_8));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_2]);
        assert_eq!(hash, Ok(hash_siblings(leaf_8, leaf_2)));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3]);
        assert_eq!(hash, Ok(leaf_12));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4]);
        assert_eq!(hash, Ok(hash_siblings(leaf_12, leaf_4)));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4, leaf_5]);
        assert_eq!(hash, Ok(hash_siblings(leaf_12, leaf_10)));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4, leaf_5, leaf_6]);
        assert_eq!(hash, Ok(hash_siblings(leaf_12, hash_siblings(leaf_10, leaf_6))));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4, leaf_5, leaf_6, leaf_7]);
        assert_eq!(hash, Ok(leaf_14));

        let hash = hash_btree(&mut vec![leaf_void, leaf_1, leaf_2, leaf_3]);
        assert_eq!(hash, Ok(hash_siblings(leaf_1, leaf_9)));

        let hash = hash_btree(&mut vec![leaf_0, leaf_void, leaf_2, leaf_3]);
        assert_eq!(hash, Ok(hash_siblings(leaf_0, leaf_9)));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_void, leaf_3]);
        assert_eq!(hash, Ok(hash_siblings(leaf_8, leaf_3)));

        let hash = hash_btree(&mut vec![leaf_0, leaf_1, leaf_2, leaf_void]);
        assert_eq!(hash, Ok(hash_siblings(leaf_8, leaf_2)));
    }
}
