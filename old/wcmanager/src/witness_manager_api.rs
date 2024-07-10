use crate::WitnessModule;

pub trait WitnessManagerAPI<'a, F> {
    fn build_wcmanager(&self) -> Box<dyn WitnessModule<'a, F> + 'a>;
    fn get_pilout_hash(&self) -> &[u8];
}
