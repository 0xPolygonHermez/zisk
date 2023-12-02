pub mod trace;
pub mod trace_col;


#[cfg(test)]
mod tests {
    use crate::trace;
    use math::fields::f64::BaseElement;
    use math::fields::CubeExtension;
    use rand::Rng;

    #[test]
    fn it_works() {
        type BaseElementExt = CubeExtension<BaseElement>;

        let mut rng = rand::thread_rng();
        let num_rows = 2_u8.pow(rng.gen_range(2..7)) as usize;

        let val_a = BaseElement::new(1);
        let val_b = BaseElementExt::new(BaseElement::new(1), BaseElement::new(1), BaseElement::new(1));

        trace!(struct Fibonacci { a: BaseElement, b: BaseElementExt, });
        let mut fibonacci = Fibonacci::new(num_rows);

        fibonacci.a.push(val_a);
        fibonacci.b.push(val_b);

        assert_eq!(fibonacci.a[0], val_a);
        assert_eq!(fibonacci.b[0], val_b);

    }
}
