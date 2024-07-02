pub mod memory;

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
