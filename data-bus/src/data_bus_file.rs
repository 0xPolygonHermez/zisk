//! A module for reading and writing DataBus information to a file.
//!
//! The `DataBusFileReader` struct provides a utility for reading DataBus information from a plain
//! text file. The `DataBusFileWriter` struct provides a utility for writing DataBus information to
//! a file.

use std::{
    fs::File,
    io::{self, Read, Write},
    str::FromStr,
};

use zisk_common::BusId;

pub struct DataBusFileReader;

impl DataBusFileReader {
    /// Reads data from a plain text file and returns a vector of `(BusId, Payload)` tuples.
    ///
    /// # File Format
    /// Each line in the file should be formatted as:
    /// ```text
    /// <BusId> <Payload1> <Payload2> ...
    /// ```
    /// - `<BusId>`: A 16-bit unsigned integer representing the bus ID.
    /// - `<PayloadN>`: A list of payload values convertible to the type `D`.
    ///
    /// # Arguments
    /// * `file_path` - The path to the plain text file.
    ///
    /// # Returns
    /// * `Result<Vec<(u16, Vec<D>)>, io::Error>`: A vector of `(BusId, Payload)` tuples or an error
    ///   if the file cannot be read or the data format is invalid.
    ///
    /// # Errors
    /// - Returns an error if the file cannot be opened or read.
    /// - Returns an error if any line is malformed (missing `BusId` or invalid payload values).
    pub fn read_from_file<D: FromStr>(file_path: &str) -> Result<Vec<(BusId, Vec<D>)>, io::Error>
    where
        D::Err: std::fmt::Display,
    {
        let mut file = File::open(file_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // Estimate the number of lines for pre-allocation
        let estimated_lines = content.lines().count();
        let mut data = Vec::with_capacity(estimated_lines);

        for (line_number, line) in content.lines().enumerate() {
            let mut parts = line.split_whitespace();

            // Parse the BusId (first token)
            let bus_id = parts
                .next()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Missing BusId on line {}", line_number + 1),
                    )
                })?
                .parse::<usize>()
                .map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid BusId on line {}: {}", line_number + 1, err),
                    )
                })?;

            // Pre-allocate payload size if possible
            let mut payload = Vec::with_capacity(parts.clone().count());

            for token in parts {
                let value = token.parse::<D>().map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid payload on line {}: {}", line_number + 1, err),
                    )
                })?;
                payload.push(value);
            }

            // Push the parsed data into the pre-allocated vector
            data.push((BusId(bus_id), payload));
        }

        Ok(data)
    }
}

/// A utility struct for writing DataBus information to a file.
pub struct DataBusFileWriter {
    file: Option<File>,
}

impl DataBusFileWriter {
    /// Creates a new `DataBusFileWriter` and opens the specified file for writing.
    ///
    /// # Arguments
    /// * `file_path` - The path to the file where data will be written.
    ///
    /// # Returns
    /// A new instance of `DataBusFileWriter`.
    pub fn new(file_path: &str) -> Result<Self, io::Error> {
        let file = File::create(file_path)?;
        Ok(Self { file: Some(file) })
    }

    /// Writes a single `(BusId, Payload)` line to the file.
    ///
    /// # Arguments
    /// * `bus_id` - The BusId to write.
    /// * `payload` - A vector of payload items to write.
    pub fn write<D: ToString>(&mut self, bus_id: u16, payload: &[D]) -> Result<(), io::Error> {
        if let Some(file) = self.file.as_mut() {
            let payload_str: String =
                payload.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(" ");
            writeln!(file, "{bus_id} {payload_str}")?;
            Ok(())
        } else {
            Err(io::Error::other("Attempted to write to a closed file."))
        }
    }

    /// Closes the file, ensuring all data is flushed to disk.
    pub fn close(&mut self) -> Result<(), io::Error> {
        if let Some(mut file) = self.file.take() {
            file.flush()?; // Ensure all buffered data is written
        }
        Ok(())
    }
}

impl Drop for DataBusFileWriter {
    /// Ensures the file is closed when the `DataBusFileWriter` is dropped.
    fn drop(&mut self) {
        let _ = self.close(); // Silently ignore any errors during drop
    }
}
