use p3_field::PrimeField;

use crate::{Expander, Planner, Surveyor};

pub trait ComponentProvider<F: PrimeField>: Send + Sync {
    fn get_surveyor(&self) -> Box<dyn Surveyor>;
    fn get_planner(&self) -> Box<dyn Planner>;
    fn get_expander(&self) -> Box<dyn Expander<F>>;
}
