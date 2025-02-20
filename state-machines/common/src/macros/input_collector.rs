#[macro_export]
macro_rules! input_collector {
    ($collector_name:ident, $op_type:expr, $data_variant:ident, $bus_id:expr) => {
        /// The `$collector_name` struct represents an input collector for `$op_type`-related operations.
        pub struct $collector_name {
            /// Collected inputs for witness computation.
            inputs: Vec<$data_variant<PayloadType>>,

            /// The number of operations to collect.
            num_operations: u64,

            /// Helper to skip instructions based on the plan's configuration.
            collect_skipper: CollectSkipper,
        }

        impl $collector_name {
            /// Creates a new `$collector_name`.
            ///
            /// # Arguments
            ///
            /// * `num_operations` - The number of operations to collect.
            /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
            ///
            /// # Returns
            /// A new `$collector_name` instance initialized with the provided parameters.
            pub fn new(num_operations: u64, collect_skipper: CollectSkipper) -> Self {
                Self { inputs: Vec::new(), num_operations, collect_skipper }
            }
        }

        impl BusDevice<PayloadType> for $collector_name {
            /// Processes data received on the bus, collecting the inputs necessary for witness computation.
            ///
            /// # Arguments
            /// * `_bus_id` - The ID of the bus (unused in this implementation).
            /// * `data` - The data received from the bus.
            ///
            /// # Returns
            /// A tuple where:
            /// - The first element indicates whether further processing should continue.
            /// - The second element contains derived inputs to be sent back to the bus (always empty).
            fn process_data(
                &mut self,
                bus_id: &data_bus::BusId,
                data: &[PayloadType],
            ) -> Option<Vec<(data_bus::BusId, Vec<PayloadType>)>> {
                debug_assert!(*bus_id == $bus_id);

                if self.inputs.len() == self.num_operations as usize {
                    return None;
                }

                let data: data_bus::ExtOperationData<u64> =
                    data.try_into().expect("Regular Metrics: Failed to convert data");

                if data_bus::OperationBusData::get_op_type(&data) as u32 != $op_type as u32 {
                    return None;
                }

                if self.collect_skipper.should_skip() {
                    return None;
                }

                if let data_bus::ExtOperationData::$data_variant(data) = data {
                    self.inputs.push(data);
                    None
                } else {
                    panic!("Expected ExtOperationData::$data_variant");
                }
            }

            /// Returns the bus IDs associated with this instance.
            ///
            /// # Returns
            /// A vector containing the connected bus ID.
            fn bus_id(&self) -> Vec<data_bus::BusId> {
                vec![$bus_id]
            }

            /// Provides a dynamic reference for downcasting purposes.
            fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }
    };
}
