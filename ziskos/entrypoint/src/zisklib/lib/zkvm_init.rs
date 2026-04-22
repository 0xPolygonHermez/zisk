#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_init")]
pub extern "C" fn zkvm_init() {
    #[cfg(all(zisk_hints, not(feature = "hints")))]
    {
        // Create ./hints directory if it doesn't exist
        let hints_dir = std::path::PathBuf::from("./hints");
        if !hints_dir.exists() {
            std::fs::create_dir_all(&hints_dir).expect("Failed to create hints directory");
        }

        // Initialize hints file
        let hints_file = std::path::PathBuf::from("./hints/block_hints.bin");
        if let Err(e) = crate::hints::init_hints_file(hints_file, None) {
            panic!("Failed to init hints, error: {}", e);
        }
    }
}

#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_deinit")]
#[allow(unused_variables)]
pub extern "C" fn zkvm_deinit(block_number: u64) {
    #[cfg(all(zisk_hints, not(feature = "hints")))]
    {
        // Close hints generation
        if let Err(e) = crate::hints::close_hints() {
            panic!("Failed to close hints, error: {}", e);
        }

        // Rename hint file
        let hints_file = std::path::PathBuf::from("./hints/block_hints.bin");
        let new_hints_file =
            std::path::PathBuf::from(format!("./hints/{}_hints.bin", block_number));
        std::fs::rename(&hints_file, &new_hints_file).unwrap();

        println!("Hints generated successfully in file {}", &new_hints_file.display());
    }
}
