use super::{CodeLine, ConstantValue, EquationConfig};
use num_bigint::BigInt;
use num_traits::One;
use regex::Regex;

#[derive(Debug, Clone)]
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

impl AdditionTerm {
    fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
    fn clear(&mut self) {
        self.terms.clear();
    }
    fn new_empty_from_term(term: &AdditionTerm) -> Self {
        Self { negative: term.negative, degree: term.degree, terms: Vec::new() }
    }
}
pub struct Equation {
    s_equation: String,
    constants: Vec<(String, ConstantValue)>,
    stack: Vec<AdditionTerm>,
    vars: Vec<String>,
    config: EquationConfig,
    chunk_size: BigInt,
    terms: Vec<Vec<AdditionTerm>>,
}

/// The `Equation` struct provides methods to parse and manipulate arithmetic equations
/// involving 256-bit integers. It supports operations such as setting constants, parsing equations,
/// generating terms, and generating code in Rust and PIL formats.
///
/// # Methods
///
/// - `new(config: &EquationConfig) -> Self`
///   - Creates a new `Equation` instance with the given configuration.
///
/// - `set_constants(&mut self, constants: &[(&str, &str)])`
///   - Sets the constants for the equation from a slice of label-value pairs.
///
/// - `get_constant_chunk(&self, id: usize, idx: usize) -> BigInt`
///   - Retrieves a specific chunk of a constant by its ID and index.
///
/// - `get_constant_id(&self, name: &str) -> Option<usize>`
///   - Retrieves the ID of a constant by its name.
///
/// - `get_constant(&self, name: &str) -> Option<(usize, BigInt)>`
///   - Retrieves the ID and value of a constant by its name.
///
/// - `get_or_create_var_id(&mut self, name: &str) -> usize`
///   - Retrieves the ID of a variable by its name, or creates a new variable if it doesn't exist.
///
/// - `join_two_ids(&mut self, id1: usize, id2: usize, op: &str, value: &BigInt) -> usize`
///   - Joins two constants by their IDs using the specified operation and value, and returns the new constant's ID.
///
/// - `get_or_create_constant_id(&mut self, name: &str, value: &BigInt, is_hex: bool) -> usize`
///   - Retrieves the ID of a constant by its name, or creates a new constant if it doesn't exist.
///
/// - `id_to_pterm(&mut self, name: &str) -> ProductTerm`
///   - Converts a constant or variable name to a `ProductTerm`.
///
/// - `push_term(&mut self, current: &mut AdditionTerm, term: &ProductTerm)`
///   - Pushes a `ProductTerm` to the current `AdditionTerm`.
///
/// - `num_to_pterm(&mut self, num: &str) -> ProductTerm`
///   - Converts a numeric string to a `ProductTerm`.
///
/// - `parse(&mut self, input: &str, constants: &[(&str, &str)])`
///   - Parses an equation string and sets the constants.
///
/// - `hex_to_bigint(s: &str) -> BigInt`
///   - Converts a hexadecimal string to a `BigInt`.
///
/// - `str_to_bigint(s: &str) -> (BigInt, bool)`
///   - Converts a string to a `BigInt`, detecting if it's in hexadecimal format.
///
/// - `generate_terms(&mut self)`
///   - Generates terms (sequence of product additions) from the parsed equation.
///
/// - `index_to_row_offset(index: usize, row: usize, terms_by_clock: usize) -> i32`
///   - Calculates the row offset (relative clock position) for a given index and row.
///
/// - `map_chunks(&mut self, terms_by_clock: usize, end_of_line: &str, last_end_of_line: &str) -> Vec<String>`
///   - Maps the terms into chunks for code generation.
///
/// - `generate_code_header(&self) -> String`
///   - Generates the header for the generated code (used to generate rust and pil).
///
/// - `generate_rust_code(&mut self, struct_name: &str, args_order: &str) -> String`
///   - Generates Rust code for the equation.
///
/// - `generate_pil_code(&mut self, const_name: &str) -> String`
///   - Generates PIL code for the equation.
///
/// - `generate_rust_code_to_file(&mut self, struct_name: &str, args_order: &str, filename: &str)`
///   - Generates Rust code and writes it to a file.
///
/// - `generate_pil_code_to_file(&mut self, const_name: &str, filename: &str)`
///   - Generates PIL code and writes it to a file.
impl Equation {
    pub fn new(config: &EquationConfig) -> Self {
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
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
    pub fn set_constants(&mut self, constants: &[(&str, &str)]) {
        self.constants = constants
            .iter()
            .map(|(label, value)| {
                let (big_int_value, is_hex) = Self::str_to_bigint(value);
                (
                    label.to_string(),
                    ConstantValue::new(
                        &big_int_value,
                        &self.chunk_size,
                        &self.config.chunks * 2,
                        is_hex,
                    ),
                )
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
        if let Some(res) = res {
            res
        } else {
            let res = self.vars.len();
            self.vars.push(name.to_string());
            res
        }
    }
    fn join_two_ids(&mut self, id1: usize, id2: usize, op: &str, value: &BigInt) -> usize {
        let is_hex = self.constants[id1].1.is_hex || self.constants[id2].1.is_hex;
        let name = format!("({}{}{})", self.constants[id1].0, op, self.constants[id2].0);
        let res = self.constants.len();
        self.constants.push((
            name.to_string(),
            ConstantValue::new(value, &self.chunk_size, &self.config.chunks * 2, is_hex),
        ));
        res
    }
    fn get_or_create_constant_id(&mut self, name: &str, value: &BigInt, is_hex: bool) -> usize {
        self.get_constant_id(name).unwrap_or_else(|| {
            let res = self.constants.len();
            self.constants.push((
                name.to_string(),
                ConstantValue::new(value, &self.chunk_size, &self.config.chunks * 2, is_hex),
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
    fn push_term(&mut self, current: &mut AdditionTerm, term: &ProductTerm) {
        if let ProductTerm::BigInt { value: term_value, id: term_id, .. } = term {
            if let Some(ProductTerm::BigInt { value: stacked_value, id: stacked_id, .. }) =
                current.terms.iter_mut().find(|t| matches!(t, ProductTerm::BigInt { .. }))
            {
                *stacked_value *= term_value.clone();
                *stacked_id = self.join_two_ids(*stacked_id, *term_id, "*", stacked_value);
                return;
            }
        }
        current.terms.push(term.clone());
    }

    fn num_to_pterm(&mut self, num: &str) -> ProductTerm {
        let value = BigInt::parse_bytes(num.as_bytes(), 10).unwrap();
        let id = self.get_or_create_constant_id(num, &value, false);
        ProductTerm::BigInt { id, index: 0, value }
    }
    pub fn parse(&mut self, input: &str, constants: &[(&str, &str)]) {
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
                self.push_term(&mut current, &pterm);
            }
            if let Some(op) = caps.name("op") {
                let negative = op.as_str() == "-";
                if negative || op.as_str() == "+" {
                    if !negative || !current.is_empty() {
                        self.stack.push(current);
                        current = AdditionTerm { negative, degree: 0, terms: Vec::new() };
                    } else {
                        current.negative = true;
                    }
                }
            }
            if let Some(num) = caps.name("num") {
                let pterm = self.num_to_pterm(num.as_str());
                self.push_term(&mut current, &pterm);
            }
        }
        if !current.is_empty() {
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
    fn calculate_limit_of_each_index(&self, term: &AdditionTerm) -> Vec<usize> {
        term.terms
            .iter()
            .map(|t| match t {
                ProductTerm::BigInt { id, .. } => self.constants[*id].1.chunks.len(),
                _ => self.config.chunks,
            })
            .collect::<Vec<usize>>()
    }
    fn next_index(&self, indexes: &mut [usize], upto: &[usize]) -> bool {
        let carry = 1;
        for i in 0..indexes.len() {
            indexes[i] += carry;
            if indexes[i] < upto[i] {
                return true;
            }
            indexes[i] = 0;
        }
        false
    }
    fn generate_terms(&mut self) {
        // Generate all terms, using stack of additions.
        for term in self.stack.iter().filter(|t| !t.terms.is_empty()) {
            let count = term.terms.len();

            // init indexes structures
            let mut indexes = vec![0; count];
            let upto = self.calculate_limit_of_each_index(term);

            // use loop because first iteration don't increment indexes
            loop {
                let mut col_index = 0;
                let mut addt = AdditionTerm::new_empty_from_term(term);
                for (i, index) in indexes.iter().enumerate() {
                    col_index += index;
                    if !self.add_prod_term(&mut addt, &term.terms[i], *index) {
                        break;
                    }
                }

                // add previous empty terms to write the current in correct position
                Self::push_term_to_col_index(&mut self.terms, col_index, addt);

                // increment to next index, if there aren't more indexes, break.
                if !self.next_index(&mut indexes, &upto) {
                    break;
                }
            }
        }
        // self.add_zero_terms();
    }
    fn push_term_to_col_index(
        terms: &mut Vec<Vec<AdditionTerm>>,
        col_index: usize,
        addt: AdditionTerm,
    ) {
        if addt.is_empty() {
            return;
        }
        while terms.len() <= col_index {
            terms.push(Vec::new());
        }
        terms[col_index].push(addt);
    }
    fn add_prod_term(&self, addt: &mut AdditionTerm, term: &ProductTerm, index: usize) -> bool {
        match term {
            ProductTerm::BigInt { id, .. } => {
                let value = self.get_constant_chunk(*id, index);
                if value == BigInt::ZERO {
                    // this full term is zero
                    addt.clear();
                    return false;
                }
                // TODO: This condition only applies if the terms
                //       distinct from one are not present
                // add the term only if different from 1
                if value != BigInt::one() || addt.terms.is_empty() {
                    addt.terms.push(ProductTerm::BigInt { id: *id, index, value });
                }
            }
            ProductTerm::Var { id, .. } => {
                addt.terms.push(ProductTerm::Var { id: *id, index });
            }
        }
        true
    }

    fn index_to_row_offset(index: usize, row: usize, terms_by_clock: usize) -> i32 {
        if terms_by_clock == 0 {
            0
        } else {
            (index / terms_by_clock) as i32 - row as i32
        }
    }
    // fn add_zero_terms(&mut self) {
    //     let total_chunks = self.config.chunks * 2;
    //     while self.terms.len() < total_chunks {
    //         self.terms.push(vec![]);
    //     }
    // }
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
            if addition_cols.is_empty() {
                output.push(format!("{}{}", 0, last_end_of_line));
                continue;
            }
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
                                format!("0x{value:X}")
                            } else {
                                format!("{value}")
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
                                // vars only has one value by clock
                                let var_terms_by_clock = 1;
                                let row_offset =
                                    Self::index_to_row_offset(*index, clock, var_terms_by_clock);
                                let comment = format!("{}[{}]", self.vars[*id], index);
                                let s_term = if row_offset == 0 {
                                    self.vars[*id].to_string()
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
                    out += end_of_line;
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
                out += &format!("// {}: 0x{:X}\n", label, value.value);
            } else {
                out += &format!("// {}: {}\n", label, value.value);
            }
        }
        out += &format!(
            "//\n// chunks:{}\n// chunk_bits:{}\n// terms_by_clock: {}\n\n",
            self.config.chunks, self.config.chunk_bits, self.config.terms_by_clock
        );
        out
    }

    pub fn generate_rust_code(&mut self, struct_name: &str, args_order: &str) -> String {
        if self.is_empty() {
            self.generate_terms();
        }

        let mut out = self.generate_code_header()
            + &format!(
                "\npub struct {struct_name} {{}}\n\nimpl {struct_name} {{\n\t#[allow(clippy::too_many_arguments)]\n\tpub fn calculate(icol: u8"
            );
        if args_order.is_empty() {
            for var in self.vars.iter() {
                out += &format!(", {var}: &[i64;16]");
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
                                "args_order:{args_order} with duplicated argument {var} for {struct_name}"
                            )
                        } else {
                            used[pos] = true;
                        }
                    }
                    None => panic!(
                        "args_order:{args_order} with unknown argument {var} for {struct_name}"
                    ),
                }
                out += &format!(", {var}: &[i64;16]");
            }
            if count < self.vars.len() {
                for (index, var) in self.vars.iter().enumerate() {
                    if used[index] {
                        continue;
                    }
                    out += &format!(", {var}: &[i64;16]");
                }
            }
        }
        out += ") -> i64 {\n\t\tmatch icol {\n";
        for (icol, col) in self.map_chunks(0, "\n", "").iter().enumerate() {
            out = out + &format!("{icol} => ") + col + ",\n";
        }
        out += "\t\t\t_ => 0,\n\t\t}\n\t}\n}\n";
        // out = out
        //     + &format!("\t\t\t_ => panic!(\"{}:", struct_name)
        //     + " error on invalid icol:{} for equation:{}\", icol, eq_index),\n\t\t}\n\t}\n}\n";
        rustfmt_wrapper::rustfmt(out).unwrap()
    }
    pub fn generate_pil_code(&mut self, const_name: &str) -> String {
        if self.is_empty() {
            self.generate_terms();
        }

        let end_of_term = format!("{: <1$}", "\n", 15 + const_name.len());
        let chunks = self.map_chunks(self.config.terms_by_clock, &end_of_term, ";");
        let mut out = self.generate_code_header()
            + &format!("\nconst expr {}_chunks[{}];\n\n", const_name, chunks.len());

        for (icol, col) in chunks.iter().enumerate() {
            if (icol % self.config.terms_by_clock) == 0 {
                out = out + &format!("// clock #{}\n\n", icol / self.config.terms_by_clock);
            }
            let label = format!("{const_name}_chunks[{icol:#2}]");
            out = out + &label + " = " + col + "\n\n";
        }
        out
    }
    pub fn generate_rust_code_to_file(
        &mut self,
        struct_name: &str,
        args_order: &str,
        filename: &str,
    ) {
        let code = self.generate_rust_code(struct_name, args_order);
        if let Err(e) = std::fs::write(filename, code) {
            eprintln!("\x1B[31mFailed to save rust code to {filename}: {e}\x1B[0m");
        } else {
            println!("Successfully wrote to rust file \x1B[32m{filename}\x1B[0m");
        }
    }
    pub fn generate_pil_code_to_file(&mut self, const_name: &str, filename: &str) {
        let code = self.generate_pil_code(const_name);
        if let Err(e) = std::fs::write(filename, code) {
            eprintln!("\x1B[31mFailed to save pil code to {filename}: {e}\x1B[0m");
        } else {
            println!("Successfully wrote to pil file \x1B[32m{filename}\x1B[0m");
        }
    }
}
