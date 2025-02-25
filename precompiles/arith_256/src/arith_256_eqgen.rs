use num_bigint::BigInt;
use num_traits::One;
use regex::Regex;
use std::path::Path;

mod code_line;
use code_line::CodeLine;

#[derive(Debug)]
enum ProductTerm {
    BigInt { id: usize, index: usize, value: BigInt },
    Var { id: usize, index: usize },
}

#[derive(Debug)]
struct AdditionTerm {
    negative: bool,
    degree: u8,
    terms: Vec<ProductTerm>,
}

struct ConstantValue {
    value: BigInt,
    is_hex: bool,
    chunks: Vec<BigInt>,
}

impl ConstantValue {
    fn new(value: BigInt, chunk_size: &BigInt, is_hex: bool) -> Self {
        let mut chunks = Vec::new();
        let mut remaining = value.clone();
        while remaining != BigInt::ZERO {
            chunks.push(&remaining % chunk_size);
            remaining = &remaining / chunk_size;
        }
        Self { value, chunks, is_hex }
    }

    fn get_chunk(&self, idx: usize) -> BigInt {
        self.chunks.get(idx).unwrap_or(&BigInt::ZERO).clone()
    }
}

#[derive(Clone, Debug)]
struct ArithEquationConfig {
    chunks: usize,
    chunk_bits: usize,
    terms_by_clock: usize,
    comment_col: usize,
}
struct ArithEquation {
    s_equation: String,
    constants: Vec<(String, ConstantValue)>,
    stack: Vec<AdditionTerm>,
    vars: Vec<String>,
    config: ArithEquationConfig,
    chunk_size: BigInt,
    terms: Vec<Vec<AdditionTerm>>,
}

impl ArithEquation {
    fn new(config: &ArithEquationConfig) -> Self {
        let chunk_size = BigInt::from(1) << config.chunk_bits;
        Self {
            s_equation: String::new(),
            constants: Vec::new(),
            stack: Vec::new(),
            vars: Vec::new(),
            terms: Vec::new(),
            config: config.clone(),
            chunk_size,
        }
    }
    fn set_constants(&mut self, constants: &[(&str, &str)]) {
        self.constants = constants
            .iter()
            .map(|(label, value)| {
                let (big_int_value, is_hex) = Self::str_to_bigint(value);
                (label.to_string(), ConstantValue::new(big_int_value, &self.chunk_size, is_hex))
            })
            .collect();
    }
    fn get_constant_chunk(&self, id: usize, idx: usize) -> BigInt {
        self.constants[id].1.get_chunk(idx)
    }
    fn get_constant_id(&self, name: &str) -> Option<usize> {
        self.constants.iter().enumerate().find(|(_, (label, _))| label == name).map(|(id, _)| id)
    }
    fn get_constant(&self, name: &str) -> Option<(usize, BigInt)> {
        self.constants
            .iter()
            .enumerate()
            .find(|(_, (label, _))| label == name)
            .map(|(id, (_, value))| (id, value.value.clone()))
    }

    fn get_or_create_var_id(&mut self, name: &str) -> usize {
        let res = self.vars.iter().position(|_name| _name == name);
        if res.is_none() {
            let res = self.vars.len();
            self.vars.push(name.to_string());
            res
        } else {
            res.unwrap()
        }
    }
    fn get_or_create_constant_id(&mut self, name: &str, value: &BigInt, is_hex: bool) -> usize {
        self.get_constant_id(name).unwrap_or_else(|| {
            let res = self.constants.len();
            self.constants.push((
                name.to_string(),
                ConstantValue::new(value.clone(), &self.chunk_size, is_hex),
            ));
            res
        })
    }
    fn id_to_pterm(&mut self, name: &str) -> ProductTerm {
        if let Some((id, value)) = self.get_constant(name) {
            ProductTerm::BigInt { id, index: 0, value: value.clone() }
        } else {
            let id = self.get_or_create_var_id(name);
            ProductTerm::Var { id, index: 0 }
        }
    }
    fn num_to_pterm(&mut self, num: &str) -> ProductTerm {
        let value = BigInt::parse_bytes(num.as_bytes(), 10).unwrap();
        let id = self.get_or_create_constant_id(num, &value, false);
        ProductTerm::BigInt { id, index: 0, value }
    }
    fn parse(&mut self, input: &str, constants: &[(&str, &str)]) {
        self.set_constants(constants);
        self.s_equation = input.to_string();
        let re =
            Regex::new(r"\s*((?P<id>[a-zA-Z_][a-zA-Z_0-9]*)|(?P<op>[\*\-\+])|(?P<num>[0-9_]+))\s*")
                .unwrap();
        let mut current = AdditionTerm { negative: false, degree: 0, terms: Vec::new() };
        for caps in re.captures_iter(input) {
            if let Some(id) = caps.name("id") {
                let pterm = self.id_to_pterm(id.as_str());
                if let ProductTerm::Var { .. } = pterm {
                    current.degree += 1;
                }
                current.terms.push(pterm);
            }
            if let Some(op) = caps.name("op") {
                let negative = op.as_str() == "-";
                if negative || op.as_str() == "+" {
                    if !negative || !current.terms.is_empty() {
                        self.stack.push(current);
                        current = AdditionTerm { negative, degree: 0, terms: Vec::new() };
                    } else {
                        current.negative = true;
                    }
                }
            }
            if let Some(num) = caps.name("num") {
                current.terms.push(self.num_to_pterm(num.as_str()));
            }
        }
        if !current.terms.is_empty() {
            self.stack.push(current);
        }
    }
    fn hex_to_bigint(s: &str) -> BigInt {
        let clean_hex = s.trim_start_matches("0x");
        BigInt::parse_bytes(clean_hex.as_bytes(), 16).unwrap()
    }
    fn str_to_bigint(s: &str) -> (BigInt, bool) {
        let is_hex = s.starts_with("0x") || s.starts_with("0X");
        if is_hex {
            return (Self::hex_to_bigint(s), true);
        }
        (BigInt::parse_bytes(s.as_bytes(), 10).unwrap(), false)
    }
    fn generate_terms(&mut self) {
        for term in self.stack.iter() {
            let count = term.terms.len();
            let mut indexes = vec![0; count];
            loop {
                let mut addt = AdditionTerm {
                    negative: term.negative,
                    degree: term.degree,
                    terms: Vec::new(),
                };
                let mut col_index = 0;
                let mut is_zero = false;
                for i in 0..count {
                    let index = indexes[i];
                    col_index += index;
                    match term.terms[i] {
                        ProductTerm::BigInt { id, .. } => {
                            let value = self.get_constant_chunk(id, index);
                            if value == BigInt::ZERO {
                                is_zero = true;
                                break;
                            }
                            if value == BigInt::one() {
                                continue;
                            }
                            addt.terms.push(ProductTerm::BigInt { id, index, value });
                        }
                        ProductTerm::Var { id, .. } => {
                            addt.terms.push(ProductTerm::Var { id, index });
                        }
                    }
                }
                if !is_zero {
                    while self.terms.len() <= col_index {
                        self.terms.push(Vec::new());
                    }
                    self.terms[col_index].push(addt);
                }
                let mut i = 0;
                loop {
                    if i == count {
                        break;
                    }
                    if indexes[i] + 1 < self.config.chunks {
                        indexes[i] += 1;
                        break;
                    } else {
                        indexes[i] = 0;
                        i += 1;
                    }
                }
                if i == count {
                    break;
                }
            }
        }
    }
    fn index_to_row_offset(index: usize, row: usize, terms_by_clock: usize) -> i32 {
        if terms_by_clock == 0 {
            0
        } else {
            (index / terms_by_clock) as i32 - row as i32
        }
    }

    fn map_chunks(
        &mut self,
        terms_by_clock: usize,
        end_of_line: &str,
        last_end_of_line: &str,
    ) -> Vec<String> {
        let mut output: Vec<String> = Vec::new();
        let mut line = CodeLine::new(terms_by_clock > 0, self.config.comment_col);
        for (icol, addition_cols) in self.terms.iter().enumerate() {
            let mut out = String::new();
            let clock = if terms_by_clock == 0 { 0 } else { icol / terms_by_clock };
            let last_j = addition_cols.len() - 1;
            for (j, addt) in addition_cols.iter().enumerate() {
                line.append(if addt.negative {
                    "- "
                } else if j > 0 {
                    "+ "
                } else {
                    "  "
                });
                for (i, term) in addt.terms.iter().enumerate() {
                    if i > 0 {
                        line.append(" * ");
                    }
                    match term {
                        ProductTerm::BigInt { value, id, index } => {
                            let s_value = if self.constants[*id].1.is_hex {
                                format!("0x{:X}", value)
                            } else {
                                format!("{}", value)
                            };
                            if terms_by_clock == 0 {
                                line.append(&s_value);
                            } else {
                                line.append_with_comments(
                                    &s_value,
                                    Some(&format!("{}[{}]", self.constants[*id].0, index)),
                                );
                            }
                        }
                        ProductTerm::Var { id, index } => {
                            if terms_by_clock == 0 {
                                line.append(&format!("{}[{}]", self.vars[*id], index));
                            } else {
                                let row_offset =
                                    Self::index_to_row_offset(*index, clock, terms_by_clock);
                                let comment = format!("{}[{}]", self.vars[*id], index);
                                let s_term = if row_offset == 0 {
                                    format!("{}", self.vars[*id])
                                } else if row_offset == -1 {
                                    format!("'{}", self.vars[*id])
                                } else if row_offset == 1 {
                                    format!("{}'", self.vars[*id])
                                } else if row_offset < 0 {
                                    format!("{}'{}", -row_offset, self.vars[*id])
                                } else {
                                    // row_offset > 0
                                    format!("{}'{}", self.vars[*id], row_offset)
                                };
                                line.append_with_comments(&s_term, Some(&comment));
                            }
                        }
                    }
                }
                if j == last_j {
                    line.append_with_comments(last_end_of_line, None);
                }
                out = out + &line.collect();
                if j != last_j {
                    out = out + end_of_line;
                }
            }
            output.push(out);
        }
        output
    }
    fn generate_code_header(&self) -> String {
        let mut out = format!("// code generated\n//\n// equation: {}\n//\n", self.s_equation);
        for (label, value) in self.constants.iter() {
            if value.is_hex {
                out = out + &format!("// {}: 0x{:X}\n", label, value.value);
            } else {
                out = out + &format!("// {}: {}\n", label, value.value);
            }
        }
        out = out
            + &format!(
                "//\n// chunks:{}\n// chunk_bits:{}\n// terms_by_clock: {}\n\n",
                self.config.chunks, self.config.chunk_bits, self.config.terms_by_clock
            );
        out
    }
    fn generate_rust_code(&mut self, eq_index: u8, args_order: &str) -> String {
        if self.terms.is_empty() {
            self.generate_terms();
        }

        let mut out = self.generate_code_header()
            + &format!(
                "\nstruct ArithEq{0} {{}}\n\nimpl ArithEq{0} {{\n\tpub fn calculate(icol: u8",
                eq_index
            );
        if args_order.is_empty() {
            for var in self.vars.iter() {
                out = out + &format!(", {}: &[i64;16]", var);
            }
        } else {
            let mut used = vec![false; self.vars.len()];
            let mut count = 0;
            for var in args_order.split(',') {
                count += 1;
                let pos = self.vars.iter().position(|v| v == var);
                match pos {
                    Some(pos) => {
                        if used[pos] {
                            panic!(
                                "args_order:{} with duplicated argument {} for ArithEq{}",
                                args_order, var, eq_index
                            )
                        } else {
                            used[pos] = true;
                        }
                    }
                    None => panic!(
                        "args_order:{} with unknown argument {} for ArithEq{}",
                        args_order, var, eq_index
                    ),
                }
                out = out + &format!(", {}: &[i64;16]", var);
            }
            if count < self.vars.len() {
                for (index, var) in self.vars.iter().enumerate() {
                    if used[index] {
                        continue;
                    }
                    out = out + &format!(", {}: &[i64;16]", var);
                }
            }
        }
        out = out + ") -> i64 {\n\t\tmatch icol {\n";
        for (icol, col) in self.map_chunks(0, "\n", "").iter().enumerate() {
            out = out + &format!("{} => ", icol) + &col + ",\n";
        }
        out = out + "\t\t\t_ => 0,\n\t\t}\n\t}\n}\n";
        rustfmt_wrapper::rustfmt(out).unwrap()
    }
    fn generate_pil_code(&mut self, eq_index: u8, const_name: &str) -> String {
        if self.terms.is_empty() {
            self.generate_terms();
        }
        let mut out = self.generate_code_header();

        let end_of_term = format!("{: <1$}", "\n", 12 + const_name.len());
        for (icol, col) in
            self.map_chunks(self.config.terms_by_clock, &end_of_term, ";").iter().enumerate()
        {
            if (icol % self.config.terms_by_clock) == 0 {
                out = out + &format!("// clock #{}\n\n", icol / self.config.terms_by_clock);
            }
            let label = format!("{}[{:#2}][{:#2}] = ", const_name, eq_index, icol);
            out = out + &label + &col + "\n\n";
        }
        out
    }
    fn generate_rust_code_to_file(&mut self, eq_index: u8, args_order: &str, filename: &str) {
        let code = self.generate_rust_code(eq_index, args_order);
        println!("Saving pil code to {} .... ", filename);
        std::fs::write(filename, code).unwrap();
    }
    fn generate_pil_code_to_file(&mut self, eq_index: u8, const_name: &str, filename: &str) {
        let code = self.generate_pil_code(eq_index, const_name);
        println!("Saving pil code to {} .... ", filename);
        std::fs::write(filename, code).unwrap();
    }
}

impl Default for ArithEquationConfig {
    fn default() -> Self {
        Self { chunks: 16, chunk_bits: 16, terms_by_clock: 2, comment_col: 30 }
    }
}

fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");

    let config =
        ArithEquationConfig { chunks: 16, chunk_bits: 16, terms_by_clock: 2, ..Default::default() };

    let mut eq = ArithEquation::new(&config);
    eq.parse(
        "x1*y1+x2-y3-q1*y2*p2_256-q0*y2",
        &[("p2_256", "0x10000000000000000000000000000000000000000000000000000000000000000")],
    );

    let rust_file = current_dir.join("helpers/arith_eq0.rs");
    eq.generate_rust_code_to_file(0, "x1,y1,x2,y2,y3,q0,q1", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/arith_eq0.pil");
    eq.generate_pil_code_to_file(0, "arith_eq", pil_file.to_str().unwrap());

    // SECP256K1

    // s - different points

    let mut eq = ArithEquation::new(&config);
    eq.parse(
        "s*x2-s*x1-y2+y1-p*q0+p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x20000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = current_dir.join("helpers/arith_eq1.rs");
    eq.generate_rust_code_to_file(1, "x1,y1,x2,y2,s,q0", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/arith_eq1.pil");
    eq.generate_pil_code_to_file(1, "arith_eq", pil_file.to_str().unwrap());

    // s - duplicate points

    let mut eq = ArithEquation::new(&config);
    eq.parse(
        "s*y1+s*y1-x1*x1-x1*x1-x1*x1+p*q0-p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x40000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = current_dir.join("helpers/arith_eq2.rs");
    eq.generate_rust_code_to_file(2, "x1,y1,s,q0", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/arith_eq2.pil");
    eq.generate_pil_code_to_file(2, "arith_eq", pil_file.to_str().unwrap());

    // x3

    let mut eq = ArithEquation::new(&config);
    eq.parse(
        "s*s-x1-x2-x3-p*q1+p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x4"),
        ],
    );

    let rust_file = current_dir.join("helpers/arith_eq3.rs");
    eq.generate_rust_code_to_file(3, "x1,x2,x3,s,q1", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/arith_eq3.pil");
    eq.generate_pil_code_to_file(3, "arith_eq", pil_file.to_str().unwrap());

    // y3

    let mut eq = ArithEquation::new(&config);
    eq.parse(
        "s*x1-s*x3-y1-y3+p*q2-p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x20000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = current_dir.join("helpers/arith_eq4.rs");
    eq.generate_rust_code_to_file(4, "x1,y1,x3,y3,s,q2", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/arith_eq4.pil");
    eq.generate_pil_code_to_file(4, "arith_eq", pil_file.to_str().unwrap());
}
