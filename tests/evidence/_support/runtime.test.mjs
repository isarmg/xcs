import assert from 'node:assert/strict';
import {test} from 'node:test';
import {mkdtemp, rm, symlink, readFile, access} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve, dirname} from 'node:path';
import {pathToFileURL} from 'node:url';
import {configureProbe, fixtureOrigin, loadPlaywright, repositoryRoot, requiredInput} from './runtime.mjs';

test('fixture URL and disposable acknowledgement are both mandatory', () => {
  assert.throws(() => fixtureOrigin({}), /ZXS_EVIDENCE_FIXTURE_URL/);
  assert.throws(() => fixtureOrigin({ZXS_EVIDENCE_FIXTURE_URL: 'http://127.0.0.1:9123'}), /DISPOSABLE/);
  for (const value of ['https://example.com:9123', 'http://localhost:9123', 'http://192.168.1.2:9123', 'file:///tmp/fixture', 'http://127.0.0.1', 'http://user:password@127.0.0.1:9123', 'http://127.0.0.1:9123/path', 'http://127.0.0.1:9123/?key=a', 'http://127.0.0.1:9123/#fragment']) {
    assert.throws(() => fixtureOrigin({ZXS_EVIDENCE_FIXTURE_URL: value, ZXS_EVIDENCE_DISPOSABLE: '1'}), /loopback/);
  }
  assert.equal(fixtureOrigin({ZXS_EVIDENCE_FIXTURE_URL: 'http://127.0.0.1:9123/', ZXS_EVIDENCE_DISPOSABLE: '1'}), 'http://127.0.0.1:9123');
  assert.equal(fixtureOrigin({ZXS_EVIDENCE_FIXTURE_URL: 'http://[::1]:9123', ZXS_EVIDENCE_DISPOSABLE: '1'}), 'http://[::1]:9123');
});
test('historical inputs and browser package never silently use private defaults', async () => {
  assert.throws(() => requiredInput('ZXS_TREE_SEED', {}), /ZXS_TREE_SEED/);
  assert.throws(() => requiredInput('INPUT', {INPUT: '  '}), /INPUT/);
  await assert.rejects(loadPlaywright({}), /ZXS_PLAYWRIGHT_MODULE/);
});
test('root is independent of nested probe depth and outputs are unique', async () => {
  const destination = await mkdtemp(resolve(tmpdir(), 'zxs-probe-config-'));
  try {
    const env = {ZXS_EVIDENCE_OUTPUT_DIR: destination};
    const script = pathToFileURL(resolve(repositoryRoot, 'tests/evidence/merge-tags-backup/web/regression-tree-probe.mjs'));
    const one = await configureProbe(script, env), two = await configureProbe(script, env);
    assert.equal(one.root, repositoryRoot.replace(/[\\/]$/, ''));
    assert.equal(dirname(one.output), destination);
    assert.notEqual(one.output, two.output);
    const relocated = await configureProbe(script, {...env, ZXS_PRODUCT_ROOT: destination});
    assert.equal(relocated.root, destination);
  } finally { await rm(destination, {recursive: true, force: true}); }
});
test('missing output, archive output, and symlink aliases are rejected', async () => {
  await assert.rejects(configureProbe(import.meta.url, {}), /OUTPUT_DIR/);
  const archive = resolve(repositoryRoot, 'tests/evidence');
  await assert.rejects(configureProbe(import.meta.url, {ZXS_EVIDENCE_OUTPUT_DIR: archive}), /outside archived/);
  await assert.rejects(configureProbe(import.meta.url, {ZXS_EVIDENCE_OUTPUT_DIR: resolve(archive, 'new-results')}), /outside archived/);
  const destination = await mkdtemp(resolve(tmpdir(), 'zxs-probe-alias-'));
  try {
    const alias = resolve(destination, 'alias');
    await symlink(archive, alias, 'dir');
    await assert.rejects(configureProbe(import.meta.url, {ZXS_EVIDENCE_OUTPUT_DIR: alias}), /inside archived/);
    const nested = resolve(alias, 'must-not-create-config-test');
    await assert.rejects(configureProbe(import.meta.url, {ZXS_EVIDENCE_OUTPUT_DIR: nested}), /inside archived/);
    await assert.rejects(access(nested), {code: 'ENOENT'});
  } finally { await rm(destination, {recursive: true, force: true}); }
});
test('all executable historical entries use guarded initialization', async () => {
  const inventory = JSON.parse(await readFile(resolve(repositoryRoot, 'tests/evidence/probes.json'), 'utf8'));
  for (const entry of inventory.probes.filter(entry => entry.kind !== 'historical-reference')) {
    const source = await readFile(resolve(repositoryRoot, 'tests/evidence', entry.path), 'utf8');
    assert.ok(source.indexOf('await configureProbe(') < source.indexOf('await loadPlaywright('), entry.path);
    if (entry.kind === 'historical-service') {
      assert.ok(source.indexOf('const historicalOrigin = fixtureOrigin();') >= 0, entry.path);
      assert.ok(source.indexOf('const historicalOrigin = fixtureOrigin();') < source.indexOf('await loadPlaywright('), entry.path);
    }
  }
});
