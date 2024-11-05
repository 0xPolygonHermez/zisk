use zisk_core::ZiskRequiredMemory;

fn prove(operations: &[ZiskRequiredMemory]) {
    // TODO: order operations
    // Get a map of the operations and a list of keys (addresses)
    let map: Map<u64, Vec<ZiskRequiredMemory>> = Map::new();
    let keys: Vec<u64> = Vec::default();
    for op in operations {
        let entry = map.entry(op.address).or_default();
        entry.push(op);
        keys.push(op.address);
    }

    // Sort the keys (addresses)
    keys.sort();

    // Fill the trace in order of address
    for key in keys {
        let ops = map.entry(key);
        let first = true;
        for op in ops {
            if op.is_write {
                panic! {"Input data operation is write"};
            }
            let mut row = InputData0Row::default();
            row.addr = F::from_canonical_u64(op.address);
            row.step = F::from_canonical_u64(op.step);
            row.sel = F::one();
            row.value[0] = F::from_canonical_u64(op.value & 0xffffffff);
            row.value[1] = F::from_canonical_u64((op.value >> 32) & 0xffffffff);
            if first {
                row.addr_changes = F::one();
                first = false;
            } else {
                row.addr_changes = F::zero();
            }
        }
    }
}
