pub trait ParseInput: Sized {
    fn parse_from_args(args: &[String]) -> Option<Self>;
}

macro_rules! impl_uint {
    ($($t:ty),*) => {
        $(impl ParseInput for $t {
            fn parse_from_args(args: &[String]) -> Option<Self> {
                args.iter().find_map(|arg| arg.parse().ok())
            }
        })*
    };
}
impl_uint!(u8, u16, u32, u64, u128);

impl ParseInput for String {
    fn parse_from_args(args: &[String]) -> Option<Self> {
        args.iter().find(|arg| !arg.starts_with("--")).cloned()
    }
}

impl ParseInput for (u64, u64) {
    fn parse_from_args(args: &[String]) -> Option<Self> {
        let ns: Vec<u64> = args.iter().filter_map(|a| a.parse().ok()).collect();
        (ns.len() >= 2).then(|| (ns[0], ns[1]))
    }
}

impl<T: std::str::FromStr> ParseInput for Vec<T> {
    fn parse_from_args(args: &[String]) -> Option<Self> {
        let v: Vec<T> = args.iter().filter_map(|a| a.parse().ok()).collect();
        (!v.is_empty()).then_some(v)
    }
}

pub fn parse_args<T: ParseInput>(default: T) -> (T, bool, bool) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let inputs = T::parse_from_args(&args).unwrap_or(default);
    let asm = args.iter().any(|arg| arg == "--asm");
    let gpu = args.iter().any(|arg| arg == "--gpu");
    (inputs, asm, gpu)
}
