import assert from 'node:assert/strict';
import fs from 'node:fs';
const [simdPath, portablePath, referencePath] = process.argv.slice(2);
const load = async (path) => {
  const bytes = fs.readFileSync(path);
  assert(WebAssembly.validate(bytes), `Engine cannot validate ${path}`);
  const { instance, module } = await WebAssembly.instantiate(bytes);
  return { exports: instance.exports, module, bytes: bytes.length };
};
const simd = await load(simdPath);
const portable = await load(portablePath);
const features = (result) => WebAssembly.Module.customSections(result.module, 'target_features').map(b => Buffer.from(b).toString('latin1')).join('');
assert(features(simd).includes('simd128'), 'SIMD artifact must declare simd128');
assert(!features(portable).includes('simd128'), 'portable reference must not declare simd128');
const reference = fs.readFileSync(referencePath, 'utf8').trim().split('\n');
assert.equal(reference.length, 72, 'all length/update-boundary cases must run');
assert.equal(new Set(reference.map(line => line.split(' ').slice(0, 2).join(' '))).size, 72, 'cases must be distinct');
const byLength = new Map();
let cases = 0;
for (const line of reference) {
  const [len, chunk, expected] = line.split(' ');
  if (byLength.has(len)) assert.equal(expected, byLength.get(len), `native update-boundary invariance length=${len}`);
  byLength.set(len, expected);
  for (const [name, result] of [['simd', simd], ['portable', portable]]) {
    const digest = Buffer.alloc(32);
    for (let word = 0; word < 8; word++) {
      digest.writeUInt32LE(result.exports.digest_word(Number(len), Number(chunk), word) >>> 0, word * 4);
    }
    assert.equal(digest.toString('hex'), expected, `${name} length=${len} chunk=${chunk}`);
  }
  cases++;
}
assert.equal(reference[0].split(' ')[2], 'af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262');
console.log(JSON.stringify({ runtime: process.version, cases, wasmComparisons: cases * 2, simdBytes: simd.bytes, portableBytes: portable.bytes, knownEmptyDigest: 'passed', nativePortableSimdEquality: 'passed', timingClaim: 'none', browserValidation: 'not performed' }, null, 2));
