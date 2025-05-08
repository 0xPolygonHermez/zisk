//! A player for replaying data on the `DataBus`.

use std::{io, str::FromStr};

use zisk_common::{BusDevice, BusId};

use crate::{DataBus, DataBusFileReader, DataBusTrait};

pub struct DataBusPlayer;

impl DataBusPlayer {
    /// Plays data on the `DataBus` from a provided data vector.
    ///
    /// # Arguments
    /// * `data_bus` - The `DataBus` to which the data is sent.
    /// * `data` - A vector of `(BusId, Payload)` tuples.
    pub fn play<D, BD: BusDevice<D>>(data_bus: &mut DataBus<D, BD>, data: Vec<(BusId, Vec<D>)>) {
        for (bus_id, payload) in data {
            <DataBus<D, BD> as DataBusTrait<D, BD>>::write_to_bus(data_bus, bus_id, &payload);
        }
    }

    /// Plays data on the `DataBus` from a file using `DataBusFileReader`.
    ///
    /// # Arguments
    /// * `file_path` - The path to the file containing the data.
    /// * `data_bus` - The `DataBus` to which the data is sent.
    ///
    /// # Returns
    /// * `Result<(), io::Error>` indicating success or failure during file reading and playing.
    pub fn play_from_file<D: FromStr, BD: BusDevice<D>>(
        data_bus: &mut DataBus<D, BD>,
        file_path: &str,
    ) -> Result<(), io::Error>
    where
        D::Err: std::fmt::Display,
    {
        let data = DataBusFileReader::read_from_file::<D>(file_path)?;
        Self::play(data_bus, data);
        Ok(())
    }
}
