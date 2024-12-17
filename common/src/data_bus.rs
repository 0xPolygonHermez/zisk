use std::collections::HashMap;

pub type Opid = u16;
pub type PayloadType = u64;

pub type MemData = [PayloadType; 4];

/// Represents a subscriber that listens to messages on the `DataBus`.
pub trait BusDevice<D> {
    fn process_data(&mut self, opid: &Opid, data: &[D]) -> Vec<(Opid, Vec<D>)>;
}

/// A bus facilitating communication between publishers and subscribers.
pub struct DataBus<D, BD: BusDevice<D>> {
    pub devices: Vec<Box<BD>>,
    devices_opid_map: HashMap<Opid, Vec<usize>>,
    omni_devices: Vec<usize>,
    pending_transfers: Vec<(Opid, Vec<D>)>,
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
            devices_opid_map: HashMap::new(),
            omni_devices: Vec::new(),
            pending_transfers: Vec::new(),
        }
    }

    pub fn connect_device(&mut self, opids: Vec<Opid>, bus_device: Box<BD>) {
        // Add the subscriber to the global subscribers list
        self.devices.push(bus_device);
        let device_idx = self.devices.len() - 1;

        for opid in opids {
            self.devices_opid_map.entry(opid).or_default().push(device_idx);
        }
    }

    pub fn connect_omni_device(&mut self, bus_device: Box<BD>) {
        // Add the subscriber to the global subscribers list
        self.devices.push(bus_device);
        let device_idx = self.devices.len() - 1;

        self.omni_devices.push(device_idx);
    }

    pub fn write_to_bus(&mut self, opid: Opid, payload: Vec<D>) {
        self.pending_transfers.push((opid, payload));

        while let Some((opid, payload)) = self.pending_transfers.pop() {
            self.route_data(opid, &payload);
        }
    }

    fn route_data(&mut self, opid: Opid, payload: &[D]) {
        // Notify specific subscribers
        if let Some(opid_devices) = self.devices_opid_map.get(&opid) {
            for &device_idx in opid_devices {
                self.pending_transfers
                    .extend(self.devices[device_idx].process_data(&opid, payload));
            }
        }

        // Notify global subscribers
        for &device_idx in &self.omni_devices {
            self.pending_transfers.extend(self.devices[device_idx].process_data(&opid, payload));
        }
    }

    pub fn debug_state(&self) {
        println!("Devices: {:?}", self.devices.len());
        println!("Devices by Opid: {:?}", self.devices_opid_map);
        println!("Global Devices: {:?}", self.omni_devices.len());
        println!("Pending Transfers: {:?}", self.pending_transfers.len());
    }
}

unsafe impl<D, BD: BusDevice<D>> Send for DataBus<D, BD> {}
