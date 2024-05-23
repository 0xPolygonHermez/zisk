## PIL2 components 

### Basic Components
|ID|subproof|pil2|tools|executor|notes|
|---|----|----|-----|----|----|
|1|basic processor|100%|-|0%||
|2|rom|100%|0%|-|tool: zkasm => pil|
|3|rom_compiled|100%|0%|-|generated from rom.json|
|4|mem|100%|-|0%||

### ZKEVM components 
|ID|subproof|pil2|tools|executor|testexec|testconst|notes|
|---|----|----|-----|----|----|----|----|
|10|main processor|90%|-|0%|no|no|
|20|rom|80%|0%|-|-|no|tool: zkasm => pil|
|40|mem|100%|-|100%|50%|no||
|50-51|mem_align|80%|-|0%|no|no|witness: 45 (pil) => 11 (pil2)|
|52|mem_align_table|80%|-|-|no|no||
|60|range_32|50%|-|0%|no|no||
|70|arith|30%|0%|0%|no|no|pending: equation generation from pil and last features added (alias free,diff points)|
|80|binary|90%|-|0%|no|no||
|81|binary (one_row)|90%|-|-|no|no|alternative, no generate executor|
|82|binary_ops_table|90%|-|-|-|no||
|100|poseidong|95%|-|0%|no|no||
|101-102|padding_pg|90%|-|0%|no|no||
|120|keccakf|90%|0%|0%|no|no|tool: script/circuit => pil|
|121|keccakf_table|90%|-|-|no|no||
|123-125|padding_kk|0%|-|0%|no|no||
|122|padding_kkbit|0%|-|0%|no|no||
|123|bits2field|-|-|-|no|no|deleted, integrated inside keccakf
|150|sha256f|80%|0%|0%|no|no|tool: script/circuit => pil|
|151|sha256f_table|90%|-|-|no|no||
|152-153|padding_sha256|0%|-|0%|no|no||
|154|padding_sha256bit|10%|-|0%|no|no||
||bits2field_sha256|-|-|-|no|no|deleted, integrated inside sha256f
|160|storage|80%|-|%0|no|no||
|161|storage_rom|100%|100%|-|no|no|tool: zkasm => pil|
|170|climb_key|90%|-|0%|no|no||
|171|climb_key_table|100%|100%|-|-|no|tool: validate/generate constants|

### TOOLS
|tool|status|notes|
|---|----|----|
|zkasm_rom2pil|0%|used to generate pil from rom.json|
|equation2pil|0%|generate equation pols with prior/next window|
|keccakf_script2pil|0%|used to generate pil from keccak script json|
|sha256_script2pil|0%|used to generate pil from sha256 script json|
|zkasm_storagerom2pil|100%|used to generate pil from storage-rom.json|
|climbkey_table|100%|used to verify complex climbkey fixed columns|
