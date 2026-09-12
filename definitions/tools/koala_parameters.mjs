// Regenerates (or checks) the Rust and C++ parameter tables from the foundation crate's
// exported JSON: `export_parameters OUT.json`, then `--check` or `--print-patch`.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const [mode, source] = process.argv.slice(2);
if (!['--check', '--print-patch'].includes(mode) || !source) {
  throw new Error('usage: node koala_parameters.mjs --check|--print-patch FOUNDATION_PARAMETERS_JSON');
}
const parameters = JSON.parse(readFileSync(source, 'utf8'));
if (parameters.upstream !== 'slop-koala-bear=6.2.2;p3-koala-bear=0.4.3-succinct'
    || parameters.modulus !== 2130706433 || parameters.exponent !== 3) {
  throw new Error('unexpected parameter source');
}
const tables = [
  ['BEGIN', parameters.begin, [4, 16]], ['PARTIAL', parameters.partial, [20]],
  ['END', parameters.end, [4, 16]], ['EXTERNAL', parameters.external, [16, 16]],
  ['INTERNAL', parameters.internal, [16, 16]],
];
for (const [, table, dimensions] of tables) {
  if (table.length !== dimensions[0] || (dimensions.length === 2 && table.some(row => row.length !== dimensions[1]))) {
    throw new Error('invalid table dimensions');
  }
  if (table.flat().some(value => !Number.isInteger(value) || value < 0 || value >= parameters.modulus)) {
    throw new Error('noncanonical parameter');
  }
}
const rust = '// Generated from pinned KoalaBear Poseidon2 parameters by definitions/tools/koala_parameters.mjs.\n'
  + tables.map(([name, table, dimensions]) => {
    const type = dimensions.length === 1 ? `[u32; ${dimensions[0]}]` : `[[u32; ${dimensions[1]}]; ${dimensions[0]}]`;
    const value = dimensions.length === 1 ? `[${table.join(', ')}]` : `[\n${table.map(row => `    [${row.join(', ')}],`).join('\n')}\n]`;
    return `pub const ${name}: ${type} = ${value};\n`;
  }).join('\n');
const cpp = '// Generated from pinned KoalaBear Poseidon2 parameters by definitions/tools/koala_parameters.mjs.\n#pragma once\n#include <stdint.h>\n\n'
  + tables.map(([name, table, dimensions]) => {
    const shape = dimensions.map(dimension => `[${dimension}]`).join('');
    const value = dimensions.length === 1 ? `{${table.join(', ')}}` : `{\n${table.map(row => `    {${row.join(', ')}},`).join('\n')}\n}`;
    return `static const uint32_t KOALA_${name}${shape} = ${value};\n`;
  }).join('\n');
const files = [
  [fileURLToPath(new URL('../src/koala_poseidon2_parameters.rs', import.meta.url)), rust],
  [fileURLToPath(new URL('../../emulator-asm/src/koala_poseidon2_parameters.hpp', import.meta.url)), cpp],
];
if (mode === '--check') {
  for (const [path, contents] of files) {
    if (readFileSync(path, 'utf8') !== contents) throw new Error(`parameter drift: ${path}`);
  }
  console.log('Rust and C++ parameters match the pinned exported source');
} else {
  console.log('*** Begin Patch\n' + files.map(([path, contents]) => `*** Add File: ${path}\n${contents.trimEnd().split('\n').map(line => `+${line}`).join('\n')}\n`).join('') + '*** End Patch');
}
