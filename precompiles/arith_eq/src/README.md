# ArithEq: Arithmetic Equations

> ⚠️ **Status**: Work in progress

## ➕ Adding New Operations

You can define new 256-bit operations by specifying up to **three equations per operation**. Multiple operations can share one or more equations.

**Important constraint:**  
To avoid unnecessary performance overhead, **all terms in an equation must be at most quadratic (degree ≤ 2)**.

### 🧮 Equation Definition and Code Generation

Equations are added in the `arith_eq_generator.rs` file. Each equation is written in a simple linear form, and associated with constant parameters.

> 🔤 Equations must be expressed as a flat sum of products, with no parentheses or spaces.

Example:

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

In this example, a equation is defined and code is generated for both Rust and PIL.

Constants like `qi` **must always be positive**, because they are split into 16-bit chunks. To ensure positivity, an **offset** is added to shift values into a valid range.

### ⚙️ Code Generation Helpers

After modifying `arith_eq_generator.rs`, regenerate the Rust and PIL code by running:
```bash
cargo run --bin arith_eq_generator --features "test_data"
```

### 📏 Range Check Guidelines

Ensure that **all intermediate values and carries are range-checked**. For operations involving `qi`, the range check rules are:
- **Most significant chunk (MSB)**: 22 bits.
- **Remaining chunks**: 16 bits each.
- **Carry values**: must be in the range from -(2^22) - 1 to 2^22.

## PIL Modification
