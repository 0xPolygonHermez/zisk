#[cfg(test)]
mod tests {

    use crate::{
        binary_basic::BinaryBasicSM,
        binary_basic_instance::{self, BinaryBasicInstance},
        binary_basic_table::BinaryBasicTableSM,
        BinarySM, ADD_OP,
    };

    use data_bus::{DataBus, DataBusPlayer, DataBusTrait};
    use fields::Goldilocks;
    use once_cell::sync::Lazy;
    use pil_std_lib::Std;
    use proofman_common::{
        MemoryHandler, PreCalculate, ProofCtx, ProofType, SetupCtx, VerboseMode::Info,
    };
    use rayon::ThreadPoolBuilder;
    use std::{any::Any, collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};
    use zisk_common::{
        ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType, OperationBusData, Plan,
        OPERATION_BUS_ID,
    };
    use zisk_core::{zisk_ops::OpType, ZiskOperationType};
    use zisk_pil::BinaryTrace;

    type F = Goldilocks;

    static TEST_CTX: Lazy<(Arc<ProofCtx<F>>, Arc<SetupCtx<F>>)> = Lazy::new(|| {
        let pk_path =
            PathBuf::from_str("/home/xavi/dev/zisk/build/provingKey").expect("Invalid path");
        let pctx =
            Arc::new(ProofCtx::<F>::create_ctx(pk_path, HashMap::new(), false, false, Info, None));
        let sctx = Arc::new(SetupCtx::new(&pctx.global_info, &ProofType::Basic, false, false));
        (pctx, sctx)
    });

    #[test]
    fn test_binary_data_counter() {
        let (pctx, sctx) = (&TEST_CTX.0, &TEST_CTX.1);

        let std_lib = Std::new(pctx.clone(), sctx.clone());

        let binary_sm = BinarySM::new(std_lib);

        let binary_bus_device =
            <BinarySM<F> as zisk_common::ComponentBuilder<F>>::build_counter(&binary_sm);

        let mut data_bus = DataBus::<u64, _>::new();
        data_bus.connect_device(Some(OPERATION_BUS_ID.0), binary_bus_device);

        let data = [(
            OPERATION_BUS_ID,
            OperationBusData::from_values(ADD_OP, OpType::Binary as u64, 1, 2).into(),
        )];

        DataBusPlayer::play(&mut data_bus, &data);

        let binary_counter = data_bus.devices.remove(0).1;

        let binary_planner =
            <BinarySM<F> as zisk_common::ComponentBuilder<F>>::build_planner(&binary_sm);

        let chunk = ChunkId(0);

        let plans = binary_planner.plan(vec![(chunk, binary_counter)]);

        println!("Plan: {:?}", plans);
    }

    fn create_ictx(chunk: ChunkId, num_operations: u64) -> InstanceCtx {
        let check_point = zisk_common::CheckPoint::Single(chunk);
        let with_adds = true;
        let collect_skipper = CollectSkipper::new(0);
        let collect_info = HashMap::from([(chunk, (num_operations, collect_skipper))]);

        let meta: Box<dyn Any> = Box::new((with_adds, collect_info));

        let plan = Plan::new(
            BinaryTrace::<usize>::AIRGROUP_ID,
            BinaryTrace::<usize>::AIR_ID,
            None,
            InstanceType::Instance,
            check_point,
            PreCalculate::None,
            Some(meta),
        );

        InstanceCtx { plan, global_id: 0 }
    }

    #[test]
    fn test_binary_data_collector() {
        let (pctx, sctx) = (&TEST_CTX.0, &TEST_CTX.1);

        let chunk = ChunkId(0);
        let num_operations = 1u64 << 22;

        let ictx = create_ictx(chunk, num_operations);

        let binary_basic_sm = BinaryBasicSM::new(BinaryBasicTableSM::new());
        let binary_basic_table_sm = BinaryBasicTableSM::new();
        let binary_instance =
            BinaryBasicInstance::new(binary_basic_sm, binary_basic_table_sm, ictx);

        let row = [(
            OPERATION_BUS_ID,
            OperationBusData::from_values(ADD_OP, ZiskOperationType::Binary as u64, 1, 2).into(),
        )];

        let threads_samples = [1, 2, 4, 8, 16];
        let repetitions = 3;

        let mut results_ms = vec![];

        let buffer_pool = Arc::new(MemoryHandler::new(1, 4_000_000_000));

        let collect_skipper = CollectSkipper::new(0);
        let collect_info = HashMap::from([(chunk, (num_operations, collect_skipper))]);

        let mut sizes = vec![0; collect_info.keys().len()];

        let mut keys: Vec<_> = collect_info.keys().collect();
        keys.sort();

        // Step 2: Iterate in sorted key order
        for (idx, key) in keys.iter().enumerate() {
            let value = collect_info.get(key).unwrap();
            sizes[idx] = value.0 as usize;
        }
        println!("Sizes: {:?}", sizes);

        for num_threads in threads_samples {
            let mut collect_ms = 0;
            let mut witness_ms = 0;

            let pool = ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap();
            pool.install(|| {
                for _ in 0..repetitions {
                    let buffer = buffer_pool.take_buffer();
                    let trace = BinaryTrace::new_from_vec(buffer);

                    *binary_instance.trace_split.lock().unwrap() =
                        Some(trace.to_split_struct(&sizes));

                    let binary_collector =
                        <binary_basic_instance::BinaryBasicInstance<F> as zisk_common::Instance<
                            F,
                        >>::build_inputs_collector(&binary_instance, chunk)
                        .expect("Failed to build inputs collector");

                    let mut data_bus = DataBus::<u64, _>::new();
                    data_bus.connect_device(Some(0), Some(binary_collector));

                    let timer_collect = std::time::Instant::now();
                    DataBusPlayer::play_repeat(&mut data_bus, &row, num_operations as usize);
                    collect_ms += timer_collect.elapsed().as_millis();

                    let mut binary_collectors = data_bus.into_devices(false);
                    let binary_collector = binary_collectors.remove(0).1.unwrap();

                    let timer_witness = std::time::Instant::now();
                    let result = binary_instance
                        .compute_witness(
                            pctx,
                            sctx,
                            vec![(0, binary_collector)],
                            buffer_pool.as_ref(),
                        )
                        .expect("Failed to compute witness");

                    buffer_pool.release_buffer(result.trace);
                    witness_ms += timer_witness.elapsed().as_millis();
                }
            });

            results_ms.push((num_threads, collect_ms / repetitions, witness_ms / repetitions));
        }

        for values in results_ms {
            println!(
                "Threads: {}, Collect: {}ms, Witness: {}ms, Total: {}ms",
                values.0,
                values.1,
                values.2,
                values.1 + values.2
            );
        }
    }
}
