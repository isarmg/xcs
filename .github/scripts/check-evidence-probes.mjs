#!/usr/bin/env node
// Static only: never import or execute archived browser probes.
import assert from 'node:assert/strict';
import {readFile, readdir, access} from 'node:fs/promises';
import {resolve, relative} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const root = fileURLToPath(new URL('../../', import.meta.url));
const evidence = resolve(root, 'test/evidence');
async function walk(directory) {
  const paths = [];
  for (const entry of await readdir(directory, {withFileTypes: true})) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) paths.push(...await walk(path));
    else if (entry.name.endsWith('.mjs')) paths.push(path);
  }
  return paths;
}
const paths = await walk(evidence);
const inventory = JSON.parse(await readFile(resolve(evidence, 'probes.json'), 'utf8'));
assert.equal(inventory.schema, 1);
assert.equal(inventory.maintained_acceptance, 'tools/browser-smoke');
assert.deepEqual(inventory.probes.map(probe => probe.path).sort(), paths.map(path => relative(evidence, path).split('\\').join('/')).filter(path => !path.startsWith('_support/')).sort(), 'Every historical script must be classified exactly once');
for (const path of paths) {
  const source = await readFile(path, 'utf8');
  for (const [, specifier] of source.matchAll(/from\s*['"](\.[^'"]+)['"]/g)) {
    await access(fileURLToPath(new URL(specifier, pathToFileURL(path))));
  }
  const checked = spawnSync(process.execPath, ['--check', path], {encoding: 'utf8'});
  assert.equal(checked.status, 0, `${path}: ${checked.stderr || checked.error}`);
}
for (const probe of inventory.probes) {
  assert.ok(['historical-reference', 'historical-service', 'historical-source-comparison', 'historical-intercepted-wasm'].includes(probe.kind), probe.path);
  assert.ok(probe.requirements.length > 20);
  const source = await readFile(resolve(evidence, probe.path), 'utf8');
  assert.doesNotMatch(source, /(?:\/mnt\/|\/home\/|\/Users\/|[A-Z]:\\\\)/, `${probe.path}: private absolute path`);
  assert.doesNotMatch(source, /https?:\/\/(?:127\.0\.0\.1|localhost):\d+/, `${probe.path}: implicit live-service origin`);
  assert.doesNotMatch(source, /['"`]\/tmp\//, `${probe.path}: implicit historical input/output`);
  if (probe.kind === 'historical-reference') {
    assert.equal(createHash('sha256').update(source).digest('hex'), probe.sha256, `${probe.path}: protected historical G4 reference changed`);
  } else {
    assert.match(source, /const probe = await configureProbe\(import.meta.url\);/);
    assert.match(source, /await loadPlaywright\(\)/);
    assert.doesNotMatch(source, /import\([^)]*playwright|from ['"][^'"]*playwright/i);
    assert.doesNotMatch(source, /(?:writeFile|screenshot)\([^\n]*\$\{root\}\/tests?\/evidence/);
    if (probe.kind === 'historical-service') assert.match(source, /fixtureOrigin\(\)/);
  }
}
const tests = spawnSync(process.execPath, ['--test', resolve(evidence, '_support/runtime.test.mjs')], {stdio: 'inherit'});
assert.equal(tests.status, 0, 'Historical probe configuration tests failed');
console.log(`PASS static syntax, portable configuration and inventory for ${inventory.probes.length} historical modules; no browser acceptance executed`);
