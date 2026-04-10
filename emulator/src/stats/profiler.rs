//! Call path profiler for tracking function call hierarchies
//!
//! This module provides functionality to track and profile function call paths
//! using a compressed representation for efficient memory usage.

use flate2::{write::GzEncoder, Compression};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::RegionsOfInterest;

/// Profiler for tracking call paths and building a call hierarchy tree
#[derive(Debug)]
pub struct CallPathProfiler {
    /// Current call path (compressed using base64 encoding)
    call_path: String,
    /// Maps call paths to their index in the call_stack_table
    prefix_table: HashMap<String, usize>,
    /// Stack of indices into call_stack_table representing current call path
    prefix_stack: Vec<usize>,
    /// Table of call stack entries: (roi_index, parent_index)
    stack_table: Vec<(usize, Option<usize>)>,
    /// Samples, first element metric (e.g., cost), second element index into stack_table
    samples: Vec<(u64, usize)>,
    ram_usage: Vec<u64>,
}

impl Default for CallPathProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl CallPathProfiler {
    /// Creates a new CallPathProfiler
    pub fn new() -> Self {
        Self {
            call_path: String::with_capacity(4 * 1024),
            prefix_table: HashMap::new(),
            stack_table: Vec::with_capacity(4 * 1024),
            prefix_stack: Vec::with_capacity(1024),
            samples: Vec::with_capacity(128 * 1024),
            ram_usage: Vec::with_capacity(128 * 1024),
        }
    }

    /// Converts a u64 value to a 3-character base64 representation
    ///
    /// This uses 18 bits (3 * 6 bits) to encode the value into 3 base64 characters
    /// using a URL-safe character set.
    pub fn hex_to_base64_short(value: u64) -> [char; 3] {
        // Base64 character set (URL-safe variant)
        const CHARSET: [char; 64] = [
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q',
            'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
            'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y',
            'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', '_',
        ];

        // Extract 18 bits (3 * 6 bits for 3 base64 chars)
        let masked = value & 0x3FFFF; // 18 bits mask

        // Convert to 3 base64 characters (6 bits each)
        [
            CHARSET[((masked >> 12) & 0x3F) as usize],
            CHARSET[((masked >> 6) & 0x3F) as usize],
            CHARSET[(masked & 0x3F) as usize],
        ]
    }

    /// Pushes a new ROI index onto the call path
    pub fn push_call_path(&mut self, roi_index: usize, metric: u64, ram_usage: u64) {
        self.internal_push_call_path(roi_index);
        self.add_call_path_sample(metric, ram_usage);
    }

    /// Internal implementation of push_call_path
    fn internal_push_call_path(&mut self, roi_index: usize) {
        self.call_path.extend(Self::hex_to_base64_short(roi_index as u64));
        // Avoid cloning if key already exists (common case) and return the index
        if let Some(&existing_index) = self.prefix_table.get(&self.call_path) {
            self.prefix_stack.push(existing_index);
        } else {
            let new_index = self.stack_table.len();
            self.prefix_table.insert(self.call_path.clone(), new_index);
            self.stack_table.push((roi_index, self.prefix_stack.last().copied()));
            self.prefix_stack.push(new_index);
        }
    }

    /// Pops the last ROI from the call path
    pub fn pop_call_path(&mut self, metric: u64, ram_usage: u64) {
        self.internal_pop_call_path();
        self.add_call_path_sample(metric, ram_usage);
    }

    /// Internal implementation of pop_call_path
    fn internal_pop_call_path(&mut self) {
        assert!(self.call_path.len() >= 3);
        self.call_path.truncate(self.call_path.len() - 3);
        self.prefix_stack.pop();
    }

    /// Adds a sample cost to the current call path entry
    pub fn add_call_path_sample(&mut self, metric: u64, ram_usage: u64) {
        if let Some(&index) = self.prefix_stack.last() {
            self.samples.push((metric, index));
            self.ram_usage.push(ram_usage);
        }
    }

    /// Updates the call path by replacing the last ROI index
    ///
    /// This is used for tail calls where we need to update the current
    /// function being tracked without adding a new level to the call stack.
    /// Assumes call_path.len() >= 3 (always true for tail calls)
    pub fn update_call_path(&mut self, roi_index: usize, metric: u64, ram_usage: u64) {
        self.internal_pop_call_path();
        self.internal_push_call_path(roi_index);
        self.add_call_path_sample(metric, ram_usage);
    }

    /// Returns a reference to the current call path string
    pub fn call_path(&self) -> &str {
        &self.call_path
    }

    /// Returns a reference to the call roots map
    pub fn call_roots(&self) -> &HashMap<String, usize> {
        &self.prefix_table
    }

    /// Saves the profiler data to a file in Firefox Profiler format
    ///
    /// # Arguments
    /// * `filename` - Output filename (.json or .gz for compressed)
    /// * `rois` - Array of RegionsOfInterest to extract function names
    ///
    /// # Format
    /// The output follows the Firefox Profiler format (version 31, preprocessedProfileVersion 61) with:
    /// - **shared.stringArray**: ROI names extracted from the rois parameter
    /// - **shared.frameTable**: 1:1 correspondence with stringArray
    /// - **shared.stackTable**: Information from self.stack_table where:
    ///   - `frame`: ROI index
    ///   - `prefix`: Parent stack index or null
    /// - **threads[0].samples**: Samples where:
    ///   - `timeDeltas`: Time differences between consecutive samples (first is 0.0)
    ///   - `stack`: Stack index
    ///
    /// # Time Deltas
    /// The profiler automatically converts absolute time values to deltas by calculating
    /// the difference between consecutive samples. The first sample always has a delta of 0.0.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::error::Error;
    /// # use ziskemu::{CallPathProfiler, RegionsOfInterest};
    /// # fn example() -> Result<(), Box<dyn Error>> {
    /// # let profiler = CallPathProfiler::new();
    /// # let rois: Vec<RegionsOfInterest> = vec![];
    /// profiler.save_to_file("profile.json", &rois)?;
    ///
    /// // For compressed output:
    /// profiler.save_to_file("profile.json.gz", &rois)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns an IO error if the file cannot be created or written
    pub fn save_to_file(&self, filename: &str, rois: &[RegionsOfInterest]) -> std::io::Result<()> {
        // Build string array from ROI names
        let string_array: Vec<&str> = rois.iter().map(|roi| roi.name.as_ref()).collect();

        // Build frame table (1:1 with string array)
        let frame_func: Vec<usize> = (0..string_array.len()).collect();

        // Build stack table from self.stack_table
        let stack_frame: Vec<usize> = self.stack_table.iter().map(|(frame, _)| *frame).collect();
        let stack_prefix: Vec<Option<usize>> =
            self.stack_table.iter().map(|(_, prefix)| *prefix).collect();

        // Build samples with time deltas
        let sample_stack: Vec<usize> = self.samples.iter().map(|(_, stack)| *stack).collect();

        let mut ram_usage: Vec<i64> = Vec::with_capacity(self.ram_usage.len());
        let mut allocations: Vec<u64> = Vec::with_capacity(self.ram_usage.len());
        let mut absolute_times: Vec<f64> = Vec::with_capacity(self.ram_usage.len());
        let mut last_usage = 0i64;
        for (index, total_bytes) in self.ram_usage.iter().enumerate() {
            let delta = *total_bytes as i64 - last_usage;
            if delta > 0 {
                ram_usage.push(delta);
                allocations.push(1);
                absolute_times.push(self.samples[index].0 as f64 / 1000.0);
                last_usage = *total_bytes as i64;
            }
        }

        // Calculate time deltas (differences between consecutive samples)
        let samples_count = self.samples.len().saturating_sub(1);
        let mut time_deltas: Vec<f64> = Vec::with_capacity(self.samples.len());
        let mut last_time = 0u64;
        for (time, _) in self.samples.iter() {
            let delta = (*time - last_time) as f64;
            time_deltas.push(delta / 1000.0);
            last_time = *time;
        }

        // Create Firefox Profiler compatible JSON structure
        let profiler_data = json!({
            "meta": {
                "interval": 1,
                "startTime": 0,
                "extensions": {
                    "id": Vec::<String>::new(),
                    "name": Vec::<String>::new(),
                    "baseURL": Vec::<String>::new(),
                    "length": 0
                },
                "processType": 0,
                "product": "ZiskEmu Profiler",
                "debug": false,
                "version": 31,
                "categories": [
                    { "color": "blue", "name": "General", "subcategories": ["Other"] },
                    { "color": "grey", "name": "Other", "subcategories": ["Other"] }
                ],
                "preprocessedProfileVersion": 61,
                "symbolicated": true,
                "markerSchema": [],
                "sampleUnits": {
                    "eventDelay": "ms",
                    "threadCPUDelta": "µs",
                    "time": "ms"
                }
            },
            "libs": [],
            "pages": [],
            "profilerOverhead": [],
            "profilingLog": json!({}),
            "profileGatheringLog": json!({}),
            "counters": [
                {
                    "name": "Memory",
                    "category": "Memory",
                    "description": "used bytes",
                    "pid": "1",
                    "mainThreadIndex": 0,
                    "samples": {
                        "time":   absolute_times,
                        "number": allocations,
                        "count":  ram_usage,
                        "length": ram_usage.len()
                    }
                }
            ],
            "shared": {
                "stringArray": string_array,
                "sources": {
                    "length": 0,
                    "filename": Vec::<String>::new(),
                    "id": Vec::<u32>::new(),
                    "startLine": Vec::<u32>::new(),
                    "startColumn": Vec::<u32>::new(),
                    "sourceMapURL": Vec::<Option<String>>::new()
                },
                "stackTable": {
                    "frame": stack_frame,
                    "prefix": stack_prefix,
                    "length": stack_prefix.len()
                },
                "frameTable": {
                    "address": vec![-1; frame_func.len()],
                    "inlineDepth": vec![0; frame_func.len()],
                    "category": vec![0; frame_func.len()],
                    "subcategory": vec![0; frame_func.len()],
                    "func": frame_func.clone(),
                    "nativeSymbol": vec![Value::Null; frame_func.len()],
                    "innerWindowID": vec![0; frame_func.len()],
                    "line": vec![Value::Null; frame_func.len()],
                    "column": vec![Value::Null; frame_func.len()],
                    "length": frame_func.len()
                },
                "funcTable": {
                    "name": frame_func,
                    "isJS": vec![false; string_array.len()],
                    "relevantForJS": vec![false; string_array.len()],
                    "resource": vec![-1; string_array.len()],
                    "source": vec![Value::Null; string_array.len()],
                    "lineNumber": vec![Value::Null; string_array.len()],
                    "columnNumber": vec![Value::Null; string_array.len()],
                    "length": string_array.len()
                },
                "resourceTable": {
                    "type": Vec::<u32>::new(),
                    "lib": Vec::<i32>::new(),
                    "name": Vec::<u32>::new(),
                    "host": Vec::<Option<u32>>::new(),
                    "length": 0
                },
                "nativeSymbols": {
                    "libIndex": Vec::<u32>::new(),
                    "address": Vec::<i64>::new(),
                    "name": Vec::<u32>::new(),
                    "functionSize": Vec::<u64>::new(),
                    "length": 0
                }
            },
            "threads": [{
                "name": "ZiskEmu",
                "isMainThread": true,
                "processType": "default",
                "processName": "ZiskEmu",
                "processStartupTime": 0,
                "processShutdownTime": Value::Null,
                "registerTime": 0,
                "unregisterTime": Value::Null,
                "tid": 1,
                "pid": "1",
                "pausedRanges": [],
                "markers": {
                    "data": Vec::<Value>::new(),
                    "name": Vec::<u32>::new(),
                    "startTime": Vec::<f64>::new(),
                    "endTime": Vec::<f64>::new(),
                    "phase": Vec::<u32>::new(),
                    "category": Vec::<u32>::new(),
                    "length": 0
                },
                "samples": {
                    "weightType": "tracing-ms",
                    "weight": time_deltas[1..].to_vec(),
                    "stack": sample_stack[..samples_count].to_vec(),
                    "timeDeltas": time_deltas[0..samples_count].to_vec(),
                    "threadCPUDelta": vec![Value::Null; samples_count],
                    "eventDelay": vec![Value::Null; samples_count],
                    "length": samples_count
                }
            }]
        });

        // Serialize to JSON
        let json_string = serde_json::to_string(&profiler_data)?;

        // Determine if we should compress based on file extension
        let path = Path::new(filename);
        let is_gzipped = path.extension().and_then(|s| s.to_str()) == Some("gz");

        if is_gzipped {
            // Write compressed file
            let file = File::create(filename)?;
            let mut encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
            encoder.write_all(json_string.as_bytes())?;
            encoder.finish()?;
        } else {
            // Write uncompressed JSON file
            let file = File::create(filename)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(json_string.as_bytes())?;
        }

        Ok(())
    }
}
