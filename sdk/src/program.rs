// pub struct ProgramId {
//     pub hash_id: &'static str,
//     pub program_name: &'static str,
// }

// pub struct Elf {
//     pub data: &'static [u8],
// }

// pub struct GuestProgram {
//     pub program_id: ProgramId,
//     pub elf: Elf,
// }

// impl GuestProgram {
//     /// Load a guest program from a URI (file path or `http(s)://` URL).
//     pub fn from_uri(uri: impl Into<String>) -> anyhow::Result<Self> {
//         let _uri = uri.into();
//         todo!("Load guest program from URI")
//     }
// }
