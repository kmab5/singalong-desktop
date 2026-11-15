import { spawnSync } from 'node:child_process';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { compile } from 'json-schema-to-typescript';
import Ajv from 'ajv';
import standaloneCode from 'ajv/dist/standalone/index.js';
import { _ } from 'ajv/dist/compile/codegen/index.js';

const result = spawnSync('cargo', ['run', '--quiet', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '-p', 'singalong-core', '--example', 'export_contract'], { encoding: 'utf8' });
if (result.status !== 0) throw new Error(result.stderr);
const schema = JSON.parse(result.stdout);
// Precompile validation: the production CSP deliberately forbids unsafe-eval.
const ajv = new Ajv({ strict: false, strictNumbers: true, code: {
  source: true, esm: true,
  formats: _`({"uint64-decimal": value => /^(0|[1-9][0-9]{0,19})$/.test(value) && BigInt(value) <= 18446744073709551615n})`,
} });
ajv.addFormat('uint64-decimal', value => /^(0|[1-9][0-9]{0,19})$/.test(value) && BigInt(value) <= 18446744073709551615n);
// Numeric bounds and finite-number checks are already encoded by JSON Schema.
ajv.addFormat('uint8', true);
ajv.addFormat('double', true);
const validate = ajv.compile({ $ref: '#/definitions/RuntimeSnapshot', definitions: schema.definitions });
const files = new Map([
  ['src/bridge/contract.schema.json', JSON.stringify(schema, null, 2) + '\n'],
  ['src/bridge/types.ts', await compile(schema, 'Contract', { bannerComment: '/* Generated from Rust. Run npm run contract:generate; do not edit. */', additionalProperties: false })],
  ['src/bridge/validate.generated.js', '// Generated from the Rust contract; do not edit.\n' + standaloneCode(ajv, validate) + '\n'],
]);
await mkdir('src/bridge', { recursive: true });
for (const [path, content] of files) {
  if (process.argv.includes('--check')) {
    if (await readFile(path, 'utf8') !== content) throw new Error(`Contract drift: ${path}`);
  } else await writeFile(path, content);
}
