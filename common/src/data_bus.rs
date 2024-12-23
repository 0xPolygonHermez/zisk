use std::collections::HashMap;

pub type BusId = u16;
pub type PayloadType = u64;

pub type MemData = [PayloadType; 4];

/// Represents a subscriber that listens to messages on the `DataBus`.
pub trait BusDevice<D> {
    fn process_data(&mut self, bus_id: &BusId, data: &[D]) -> (bool, Vec<(BusId, Vec<D>)>);
}

/// A bus facilitating communication between publishers and subscribers.
pub struct DataBus<D, BD: BusDevice<D>> {
    pub devices: Vec<Box<BD>>,
    devices_bus_id_map: HashMap<BusId, Vec<usize>>,
    omni_devices: Vec<usize>,
    pending_transfers: Vec<(BusId, Vec<D>)>,
}

impl<D, BD: BusDevice<D>> Default for DataBus<D, BD> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D, BD: BusDevice<D>> DataBus<D, BD> {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            devices_bus_id_map: HashMap::new(),
            omni_devices: Vec::new(),
            pending_transfers: Vec::new(),
        }
    }

    pub fn connect_device(&mut self, bus_ids: Vec<BusId>, bus_device: Box<BD>) {
        // Add the subscriber to the global subscribers list
        self.devices.push(bus_device);
        let device_idx = self.devices.len() - 1;

        for bus_id in bus_ids {
            self.devices_bus_id_map.entry(bus_id).or_default().push(device_idx);
        }
    }

    pub fn connect_omni_device(&mut self, bus_device: Box<BD>) {
        // Add the subscriber to the global subscribers list
        self.devices.push(bus_device);
        let device_idx = self.devices.len() - 1;

        self.omni_devices.push(device_idx);
    }

    pub fn write_to_bus(&mut self, bus_id: BusId, payload: Vec<D>) -> bool {
        self.pending_transfers.push((bus_id, payload));

        while let Some((bus_id, payload)) = self.pending_transfers.pop() {
            if self.route_data(bus_id, &payload) {
                return true;
            }
        }

        false
    }

    fn route_data(&mut self, bus_id: BusId, payload: &[D]) -> bool {
        // Notify specific subscribers
        if let Some(bus_id_devices) = self.devices_bus_id_map.get(&bus_id) {
            for &device_idx in bus_id_devices {
                let (end, result) = self.devices[device_idx].process_data(&bus_id, payload);
                self.pending_transfers.extend(result);

                if end {
                    return true;
                }
            }
        }

        // Notify global subscribers
        for &device_idx in &self.omni_devices {
            let (end, result) = self.devices[device_idx].process_data(&bus_id, payload);
            self.pending_transfers.extend(result);

            if end {
                return true;
            }
        }

        false
    }

    pub fn debug_state(&self) {
        println!("Devices: {:?}", self.devices.len());
        println!("Devices by bus id: {:?}", self.devices_bus_id_map);
        println!("Global Devices: {:?}", self.omni_devices.len());
        println!("Pending Transfers: {:?}", self.pending_transfers.len());
    }

    pub fn detach_first_device(&mut self) -> Option<Box<BD>> {
        self.devices.pop()
    }
    
    pub fn detach_devices(&mut self) -> Vec<Box<BD>> {
        std::mem::take(&mut self.devices)
    }
}

unsafe impl<D, BD: BusDevice<D>> Send for DataBus<D, BD> {}
