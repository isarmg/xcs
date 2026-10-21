// Configuration only: importing this module never starts a browser or service.
import {mkdir, mkdtemp, realpath} from 'node:fs/promises';
import {basename, dirname, relative, resolve, sep, isAbsolute} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

export const repositoryRoot = fileURLToPath(new URL('../../../', import.meta.url));
export function requiredInput(name, env = process.env) {
  if (!env[name]?.trim()) throw new Error(`${name} must explicitly identify the historical fixture input; see test/evidence/README.md`);
  return env[name];
}
export function fixtureOrigin(env = process.env) {
  const value = requiredInput('ZXS_EVIDENCE_FIXTURE_URL', env);
  const url = new URL(value);
  if (url.protocol !== 'http:' || !['127.0.0.1', '[::1]'].includes(url.hostname) ||
      !url.port || url.username || url.password || url.pathname !== '/' || url.search || url.hash) {
    throw new Error('ZXS_EVIDENCE_FIXTURE_URL must be an explicit HTTP loopback origin with a port, no credentials/path/query');
  }
  if (env.ZXS_EVIDENCE_DISPOSABLE !== '1') throw new Error('Set ZXS_EVIDENCE_DISPOSABLE=1 only for an independently provisioned, disposable fixture; historical probes can mutate it');
  return url.origin;
}
function within(child, parent) {
  const rel = relative(parent, child);
  return !rel || (!rel.startsWith(`..${sep}`) && rel !== '..' && !isAbsolute(rel));
}
async function canonicalDestination(path) {
  try { return await realpath(path); }
  catch (error) {
    if (error.code !== 'ENOENT') throw error;
    const parent = dirname(path);
    if (parent === path) throw error;
    return resolve(await canonicalDestination(parent), basename(path));
  }
}
export async function configureProbe(scriptUrl, env = process.env) {
  const root = await realpath(resolve(env.ZXS_PRODUCT_ROOT || repositoryRoot));
  const requested = resolve(requiredInput('ZXS_EVIDENCE_OUTPUT_DIR', env));
  const archives = [repositoryRoot, root].flatMap(base =>
    ['test/evidence', 'tests/evidence'].map(path => resolve(base, path)));
  if (archives.some(archive => within(requested, archive))) throw new Error('Output must be outside archived test/evidence');
  const destination = await canonicalDestination(requested);
  const canonicalArchives = await Promise.all(archives.map(path => realpath(path).catch(() => path)));
  if (canonicalArchives.some(archive => within(destination, archive))) throw new Error('Output symlink resolves inside archived test/evidence');
  await mkdir(destination, {recursive: true});
  // A fresh child for every invocation also avoids overwriting earlier reruns.
  const output = await mkdtemp(resolve(destination, `${basename(fileURLToPath(scriptUrl), '.mjs')}-`));
  console.log(`Historical probe output (not current acceptance): ${output}`);
  return {root, output};
}
export async function loadPlaywright(env = process.env) {
  const module = requiredInput('ZXS_PLAYWRIGHT_MODULE', env);
  return import(isAbsolute(module) ? pathToFileURL(module).href : module);
}
