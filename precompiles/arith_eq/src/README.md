# ArithEq: Arith Equations

**TODO: In construction**

## Add new 256 bit operations
It's possible to add new operations with up to 3 equations for operation.

Different operations could share more than one equations

To avoid pay an extra cost, all terms of each equation must be segon degree or less.

## Equations generation
Add the new equations to generator (`arith_eq_generator.rs`), in this file add the description of
of equations and their files names. An example:

```rust
    let mut eq = Equation::new(&config);
    eq.parse(
        "s*x2-s*x1-y2+y1-p*q0+p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x20000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("secp256k1_add.rs");
    eq.generate_rust_code_to_file("Secp256k1Add", "x1,y1,x2,y2,s,q0", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("secp256k1_add.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_add", pil_file.to_str().unwrap());
```
In this example, it's defined the equation as an expression = 0, without parentesis, only with
a sequence of products additions.

### Helpers generation

After source `arith_eq_generator.rs` was update, to regenerate rust helpers and pil, execute:

```bash
cargo run --bin arith_eq_generator --features "test_data"
```

### Defined Constants

`qi` must be a possitive number because is divided in possitive chunks of 16, for these reason use
an addition offset to be sure that always it's possitive.

### Range Check

Be carefull, that all calculations are inside range check. In case of `qi` range check it's __22 bits__
for the __most significant chunk__, and __16 bits__ for the rest of chunks. The valid range check of carry was from -(2^22)-1 to 2^22

## Pil modification


