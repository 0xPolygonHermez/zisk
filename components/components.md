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
|1|main processor|90%|-|0%|no|no|
|2|rom|80%|0%|-|-|no|tool: zkasm => pil|
|4|mem|100%|-|100%|50%|no||
|5|mem_align|80%|-|0%|no|no||
|6|range_32|50%|-|0%|no|no||
|10|arith|30%|0%|0%|no|no|pending: equation generation from pil and last features added (alias free,diff points)|
|20|binary|90%|-|0%|no|no||
|20|binary (one_row)|90%|-|-|no|no|alternative, no generate executor|
|21|binary_ops_table|90%|-|-|-|no||
|40-42|padding_pg|90%|-|0%|no|no||
|45|poseidong|95%|-|0%|no|no||
|50-52|padding_kk|0%|-|0%|no|no||
|55|padding_kkbit|0%|-|0%|no|no||
|56|keccakf|90%|0%|0%|no|no|tool: script/circuit => pil|
|57|keccakf_table|90%|-|-|no|no||
||bits2field|-|-|-|no|no|deleted, integrated inside keccakf
|60-62|padding_sha256|0%|-|0%|no|no||
|65|padding_sha256bit|10%|-|0%|no|no||
|66|sha256f|80%|0%|0%|no|no|tool: script/circuit => pil|
|67|sha256f_table|90%|-|-|no|no||
||bits2field_sha256|-|-|-|no|no|deleted, integrated inside sha256f
|90|storage|80%|-|%0|no|no||
|91|storage_rom|100%|100%|-|no|no|tool: zkasm => pil|
|92|climb_key|90%|-|0%|no|no||
|93|climb_key_table|100%|100%|-|-|no|tool: validate/generate constants|

### TOOLS
|tool|status|notes|
|---|----|----|
|zkasm_rom2pil|0%|used to generate pil from rom.json|
|equation2pil|0%|generate equation pols with prior/next window|
|keccakf_script2pil|0%|used to generate pil from keccak script json|
|sha256_script2pil|0%|used to generate pil from sha256 script json|
|zkasm_storagerom2pil|100%|used to generate pil from storage-rom.json|
|climbkey_table|100%|used to verify complex climbkey fixed columns|
