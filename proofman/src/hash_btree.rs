#[cfg(not(feature = "no_lib_link"))]
use zkevm_lib_c::ffi::goldilocks_linear_hash_c;

#[cfg(feature = "no_lib_link")]
pub fn hash_siblings(left: [u64; 4], right: [u64; 4]) -> [u64; 4] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2], left[3] + right[3]]
}

#[cfg(not(feature = "no_lib_link"))]
pub fn hash_siblings(left: [u64; 4], right: [u64; 4]) -> [u64; 4] {
    use std::ffi::c_void;

    let inputs: [u64; 8] = [left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3]];
    let mut out: [u64; 4] = [0u64; 4];

    goldilocks_linear_hash_c(inputs.as_ptr() as *mut c_void, out.as_mut_ptr() as *mut c_void);

    out
}

pub fn hash_btree_256(input_array: &mut Vec<[u64; 4]>) -> Result<[u64; 4], &'static str> {
    if input_array.is_empty() {
        return Err("Cannot hash an empty array");
    }

    if input_array.len() == 1 {
        let first_element = input_array[0];
        if first_element.iter().all(|&x| x == 0u64) {
            return Err("All elements in the array are zero");
        }
        return Ok(first_element.clone());
    }

    // Pad the input array if it has odd length
    if input_array.len() % 2 != 0 {
        input_array.push([0u64; 4]);
    }

    let mut result = Vec::with_capacity(input_array.len() / 2);

    // Process pairs of elements
    for chunk in input_array.chunks_exact(2) {
        let (left, right) = (chunk[0], chunk[1]);

        let is_left_zero = left.iter().all(|&x| x == 0u64);
        let is_right_zero = right.iter().all(|&x| x == 0u64);

        let hash = if is_left_zero && is_right_zero {
            [0u64; 4]
        } else if is_left_zero {
            right
        } else if is_right_zero {
            left
        } else {
            hash_siblings(left, right)
        };
        result.push(hash);
    }

    // Recursively call the function with the new array of hashes
    hash_btree_256(&mut result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_btree_256() {
        let leaf_void = [0, 0, 0, 0];
        let leaf_0 = [1, 1, 1, 1];
        let leaf_1 = [11, 11, 11, 11];
        let leaf_2 = [111, 111, 111, 111];
        let leaf_3 = [1111, 1111, 1111, 1111];
        let leaf_4 = [11111, 11111, 11111, 11111];
        let leaf_5 = [111111, 111111, 111111, 111111];
        let leaf_6 = [1111111, 1111111, 1111111, 1111111];
        let leaf_7 = [11111111, 11111111, 11111111, 11111111];

        let leaf_8 = hash_siblings(leaf_0, leaf_1);
        let leaf_9 = hash_siblings(leaf_2, leaf_3);
        let leaf_10 = hash_siblings(leaf_4, leaf_5);
        let leaf_11 = hash_siblings(leaf_6, leaf_7);
        let leaf_12 = hash_siblings(leaf_8, leaf_9);
        let leaf_13 = hash_siblings(leaf_10, leaf_11);
        let leaf_14 = hash_siblings(leaf_12, leaf_13);

        let hash = hash_btree_256(&mut Vec::new());
        assert_eq!(hash, Err("Cannot hash an empty array"));

        // Create test data
        let hash = hash_btree_256(&mut vec![leaf_void]);
        assert_eq!(hash, Err("All elements in the array are zero"));

        let hash = hash_btree_256(&mut vec![leaf_void, leaf_void]);
        assert_eq!(hash, Err("All elements in the array are zero"));

        let hash = hash_btree_256(&mut vec![leaf_void, leaf_void, leaf_void]);
        assert_eq!(hash, Err("All elements in the array are zero"));

        // Create test data
        let hash = hash_btree_256(&mut vec![leaf_0]);
        assert_eq!(hash, Ok(leaf_0));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1]);
        assert_eq!(hash, Ok(leaf_8));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_2]);
        assert_eq!(hash, Ok(hash_siblings(leaf_8, leaf_2)));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3]);
        assert_eq!(hash, Ok(leaf_12));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4]);
        assert_eq!(hash, Ok(hash_siblings(leaf_12, leaf_4)));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4, leaf_5]);
        assert_eq!(hash, Ok(hash_siblings(leaf_12, leaf_10)));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4, leaf_5, leaf_6]);
        assert_eq!(hash, Ok(hash_siblings(leaf_12, hash_siblings(leaf_10, leaf_6))));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_2, leaf_3, leaf_4, leaf_5, leaf_6, leaf_7]);
        assert_eq!(hash, Ok(leaf_14));

        let hash = hash_btree_256(&mut vec![leaf_void, leaf_1, leaf_2, leaf_3]);
        assert_eq!(hash, Ok(hash_siblings(leaf_1, leaf_9)));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_void, leaf_2, leaf_3]);
        assert_eq!(hash, Ok(hash_siblings(leaf_0, leaf_9)));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_void, leaf_3]);
        assert_eq!(hash, Ok(hash_siblings(leaf_8, leaf_3)));

        let hash = hash_btree_256(&mut vec![leaf_0, leaf_1, leaf_2, leaf_void]);
        assert_eq!(hash, Ok(hash_siblings(leaf_8, leaf_2)));
    }
}
