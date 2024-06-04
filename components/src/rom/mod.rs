use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RomProgram {
    pub program_lines: Vec<RomProgramLine>,
    pub labels: HashMap<String, i64>,
    pub constants: Value,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RomProgramLine {
    pub line: isize,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "lineStr")]
    pub line_str: String,
    #[serde(flatten)]
    pub program_line: Map<String, Value>,
}

#[allow(dead_code)]
impl RomProgram {
    pub fn from_json(input_json: &str) -> Result<RomProgram, serde_json::Error> {
        let parsed_json: RomProgram = serde_json::from_str(input_json)?;

        Ok(parsed_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test deserialization and parsing of JSON input
    #[test]
    fn test_parse_rom_json() {
        let input_json = r#"
            {
                "program_lines": [
                    {
                        "inSTEP": "1",
                        "setA": 1,
                        "line": 5,
                        "fileName": "rom.zkasm",
                        "lineStr": "        STEP => A"
                    },
                    {
                        "CONST": "0",
                        "assert": 1,
                        "line": 6,
                        "fileName": "rom.zkasm",
                        "lineStr": "        0 :ASSERT"
                    },
                    {
                        "inA": "1",
                        "CONST": "10",
                        "offset": 0,
                        "offsetLabel": "myvar",
                        "mOp": 1,
                        "mWR": 1,
                        "assumeFree": 0,
                        "line": 13,
                        "useCTX": 0,
                        "fileName": "rom.zkasm",
                        "lineStr": "        A + 10      :MSTORE(myvar)"
                    },
                    {
                        "freeInTag": {
                            "op": ""
                        },
                        "inFREE": "1",
                        "inFREE0": "0",
                        "setA": 1,
                        "offset": 0,
                        "offsetLabel": "myvar",
                        "mOp": 1,
                        "mWR": 0,
                        "assumeFree": 0,
                        "line": 14,
                        "useCTX": 0,
                        "fileName": "rom.zkasm",
                        "lineStr": "        $ => A      :MLOAD(myvar)"
                    },
                    {
                        "CONST": "0",
                        "setA": 1,
                        "setB": 1,
                        "setC": 1,
                        "setD": 1,
                        "setE": 1,
                        "setCTX": 1,
                        "setSP": 1,
                        "setPC": 1,
                        "line": 19,
                        "fileName": "rom.zkasm",
                        "lineStr": "        0 => A,B,C,D,E,CTX, SP, PC"
                    },
                    {
                        "freeInTag": {
                            "op": "functionCall",
                            "funcName": "beforeLast",
                            "params": []
                        },
                        "inFREE": "1",
                        "inFREE0": "0",
                        "JMP": 0,
                        "JMPZ": 0,
                        "JMPC": 0,
                        "JMPN": 1,
                        "return": 0,
                        "call": 0,
                        "free0IsByte": 0,
                        "jmpAddr": 10,
                        "jmpAddrLabel": "finalWait",
                        "elseAddr": 11,
                        "elseAddrLabel": "next",
                        "line": 22,
                        "fileName": "rom.zkasm",
                        "lineStr": "        ${beforeLast()}  : JMPN(finalWait)"
                    },
                    {
                        "JMP": 1,
                        "JMPZ": 0,
                        "JMPC": 0,
                        "JMPN": 0,
                        "return": 0,
                        "call": 0,
                        "jmpAddr": 0,
                        "jmpAddrLabel": "start",
                        "line": 24,
                        "fileName": "rom.zkasm",
                        "lineStr": "                         : JMP(start)"
                    }
                ],
                "labels": {
                    "start": 0,
                    "end": 9,
                    "finalWait": 10,
                    "failAssert": 12
                },
                "constants": {}
            }
        "#;

        let rom_program_json = RomProgram::from_json(input_json);
        assert!(rom_program_json.is_ok());

        let rom_program_json = rom_program_json.unwrap();
        assert_eq!(rom_program_json.program_lines.len(), 7);
        assert_eq!(rom_program_json.labels.len(), 4);
        assert!(rom_program_json.constants.is_object());
    }

    // Test deserialization and parsing of JSON input with wrong fields
    #[test]
    fn test_parse_input_json_wrong_fields() {
        let input_json = r#"
            {
                "wrong": "fields"
            }
        "#;

        let rom_program_json = RomProgram::from_json(input_json);
        assert!(rom_program_json.is_err());
    }
}
