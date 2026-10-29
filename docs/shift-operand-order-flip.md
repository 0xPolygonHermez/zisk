# Giro del orden de operandos de los shifts (eliminar `op_is_shift`)

> Estado: **propuesta** — pendiente de PR propia (solo este cambio).
> Fecha de la nota: 2026-07-24.

## Objetivo

Unificar las operaciones de la familia *shift* de la `BinaryExtension` con la forma de
las operaciones *plain* (tipo `signextend`), de modo que **el valor operado siempre viaje
en el operando `b` del bus** y el **parámetro pequeño** (shift amount / índice de bit) viaje
en el operando `a`.

Con eso desaparece la necesidad del selector `op_is_shift`, porque hoy su único trabajo real
es enrutar `free_in_a` a `bus-a` (shift) vs `bus-b` (plain).

## Idea (antes → después)

Hoy la reconstrucción del bus se bifurca por familia:

- **plain** (sext/rev8/…): valor (`free_in_a`) → `bus-b`; `bus-a` = el "otro" operando (0 en single-source).
- **shift**: valor (`free_in_a`) → `bus-a`; shift amount → `bus-b`.

Tras el giro (para shifts: valor → `bus-b`, amount → `bus-a`):

```
bus-b = [a0, a1]                    // siempre el valor, desde free_in_a
bus-a = free_in_b (+ parte alta)    // shift: amount ; plain: 0 (free_in_b = 0)
```

Como en plain `free_in_b = 0` y la parte alta es 0, **la misma fórmula sirve para ambos** →
se elimina `op_is_shift`:

- de la **reconstrucción** (`proves_operation`),
- del **selector** del `range_check` de la parte alta del amount (se puede range-checkear
  incondicionalmente porque en no-shift vale 0),
- de su **bit en el fold `OFFSET_AND_FLAGS`** de la tabla (el `OP` ya identifica la fila).

`op_is_combine` (pack) **no** se elimina: pack pone datos en `bus-a` y `bus-b` a la vez, no
tiene la forma "sext". Los flags de cadena (`op_is_chain`/`op_is_chain_rev`, solo full) son
independientes.

> Nota: la **tabla** interna de la binary extension no cambia su cómputo (sigue `A = byte del
> valor`, `B = amount`); lo que cambia es el mapeo `bus ↔ A/B` en el caller y qué registro
> alimenta cada operando en el resto de capas.

## Operaciones a girar (14)

Criterio: las que devuelven `true` en `opcode_is_shift`
([binary_extension.rs](../state-machines/binary/src/binary_extension.rs)), es decir, valor en
`a` y parámetro (amount / índice de bit) en `b`.

| Grupo | ZiskOp | `b` actual | RISC-V (reg-reg / inmediato) |
|---|---|---|---|
| Shift 64 | `Sll`, `Srl`, `Sra` | shift amount | `sll/srl/sra`, `slli/srli/srai` |
| Shift 32 (word) | `SllW`, `SrlW`, `SraW` | shift amount | `sllw/srlw/sraw`, `slliw/srliw/sraiw` |
| Rotate | `Rol`, `RolW`, `Ror`, `RorW` | rotate amount | `rol/rolw/ror/rorw`, `rori/roriw` |
| Bit (Zbs) | `Bclr`, `Bext`, `Binv`, `Bset` | índice de bit | `bclr/bext/binv/bset`, `bclri/bexti/binvi/bseti` |

Semántica a invertir en `op_*` (ejemplo): `op_sll(a,b) = a << (b&63)` → `b << (a&63)`.

### Caso especial: `SllUW` (`sll_u_w`, opcode `0x4f`)

- Es shift-semántica: `op_sll_u_w(a,b) = (a & 0xFFFFFFFF) << (b&0x3F)` (valor en `a`, amount en `b`).
- Catalogada `BinaryE` en [zisk_ops.rs](../core/src/zisk_ops.rs) pero **solo soportada en
  emulación (para evaluar costes); no tiene aritmetización** (su opcode `0x4f` no está en la
  tabla de la binary extension, que llega a `OP_BSET = 0x47`; el transpiler tiene ruta propia
  `self.sll_u_w(...)`).
- **No es imprescindible girarla** para eliminar `op_is_shift`, pero por coherencia con las
  instrucciones análogas **conviene cambiarla también**. Decisión pendiente al abrir la PR.

## Impacto por capa (checklist de la PR)

1. **Semántica de ops** — [core/src/ops_core.rs](../core/src/ops_core.rs) + wrappers
   `opc_*` en [core/src/ops_core_context.rs](../core/src/ops_core_context.rs): invertir `a`/`b`
   en las 14 `op_*` (+ `op_sll_u_w` si se incluye).

2. **Transpiler RISC-V→ZisK** — [transpilers/riscv/src/riscv2zisk_context.rs](../transpilers/riscv/src/riscv2zisk_context.rs):
   - Reg-reg: hoy usan `create_register_op` (compartido con add/sub/mul/div/…). **No** cambiar
     `create_register_op`; hace falta una ruta específica de shifts que intercambie `src_a`↔`src_b`.
   - Inmediatos: `immediate_op` (slli/srli/srai(+w), rori(+w), bclri/bexti/binvi/bseti):
     intercambiar reg (valor) e imm (parámetro).
   - `sll_u_w` tiene su propia ruta.

3. **Generación de assembly (ruta de EJECUCIÓN)** — [core/src/zisk_rom_2_asm.rs](../core/src/zisk_rom_2_asm.rs):
   - `store_a_in_c = true` para la familia (≈ línea 1277-1292): al pasar el valor de `a` a `b`,
     cambia la lógica de store.
   - ASM explícito por op (≈ líneas 2858-3784, ~14 ops) que lee `REG_A`/`REG_B`: intercambiar
     qué registro es valor y cuál parámetro en cada una.
   - `SllUW` (≈ línea 6406).

4. **Witness de binary extension** — [state-machines/binary/src/binary_extension.rs](../state-machines/binary/src/binary_extension.rs):
   simplificar/eliminar `a_val`/`b_val` según `op_is_shift`, y quitar `opcode_is_shift`.

5. **PIL de binary extension** — [state-machines/binary/pil/binary_extension.pil](../state-machines/binary/pil/binary_extension.pil):
   eliminar `op_is_shift` de la reconstrucción, del selector del `range_check` y del fold de flags.
   La [tabla](../state-machines/binary/pil/binary_extension_table.pil) no cambia su cómputo, pero
   revisar el bit `op_is_shift` en `OFFSET_AND_FLAGS`.

6. **Frequent-ops (frops)** — ⚠️ **lo más delicado**:
   - [state-machines/frequent-ops/src/frequent_ops_table.rs](../state-machines/frequent-ops/src/frequent_ops_table.rs)
     usa `OP_SLL/SLLW/SRA/SRL/…` directamente.
   - [tools/frops-analyzer/src/codegen.rs](../tools/frops-analyzer/src/codegen.rs) genera código
     asumiendo la estructura del resultado.
   - Binarios precomputados (`binary_basic_frops_fixed.bin`, `binary_extension_frops_fixed.bin`)
     codifican resultados por pares `(a,b)`: **regenerarlos** y revalidar el analyzer.

7. **Tests** — [riscv_ima.rs](../test-artifacts/programs/diagnostic/src/riscv_ima.rs)
   y [riscv_b.rs](../test-artifacts/programs/diagnostic/src/riscv_b.rs): los tests a nivel RISC-V
   deberían seguir pasando (la semántica RISC-V no cambia), pero revisar cualquiera que asserte
   sobre el orden de operandos ZisK o valores del bus.

## Validación

- Ejecutar la suite de emulación/prueba end-to-end (los resultados RISC-V deben ser idénticos).
- Regenerar los `*_frops_fixed.bin` y confirmar que las lookups cuadran.
- Recompilar el PIL (`compile-pil`) y correr la generación de prueba de un programa que use
  shifts, rotates y bit-ops (Zbb/Zbs) para cubrir toda la familia.

## Riesgos / notas

- El ahorro en el circuito (una columna witness `op_is_shift` + una rama de reconstrucción +
  su bit en el fold) es **modesto**; el coste dominante es la **consistencia transversal**
  (semántica ↔ asm ↔ transpiler ↔ prueba ↔ frops). El principal riesgo es una inconsistencia
  sutil entre capas.
- Mantener el giro **atómico** en la PR: semántica, transpiler (reg-reg + imm), asm y frops
  deben cambiar juntos, o la ejecución/prueba divergen.
- Confirmar la ruta de descomposición de `SllUW` antes de decidir si entra en el giro.
