pub mod pilout_proxy;
pub mod pilout {
    include!(concat!(env!("OUT_DIR"), "/pilout.rs"));
}
