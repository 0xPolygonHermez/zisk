// mod memory;
mod basic_processor;
mod component;
mod memory;
mod register;
mod rom;

// pub use memory::*;
pub use basic_processor::*;
pub use rom::*;
pub use component::*;
pub use memory::*;

#[cfg(test)]
mod tests {

    pub fn add(left: usize, right: usize) -> usize {
        left + right
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
