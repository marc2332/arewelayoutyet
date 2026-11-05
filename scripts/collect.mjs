import { readFile, readdir } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { cpus, platform } from 'node:os';
import { execFileSync } from 'node:child_process';
import { parseArgs } from 'node:util';
import { readHistory, writeHistory } from './history-data.mjs';

const libraries = new Map([['Taffy main', 'taffy'], ['Morphorm', 'morphorm'], ['Torin', 'torin'], ['Yoga', 'yoga'], ['ui_layout', 'ui_layout'], ['Clay', 'clay-layout'], ['Ply', 'ply-engine'], ['Backer', 'backer'], ['Cephal', 'cephal'], ['keru_layout', 'keru_layout']]);
const upstream = '4e59968eda29ceeb0bbaeb3993c8cd38baa3d0a6';
const { values } = parseArgs({ options: {
  criterion: { type: 'string', default: 'benchmarks/target/criterion' },
  history: { type: 'string', default: 'site/data/history.json' },
} });
const files = await readdir(values.criterion, { recursive: true });
const results = [];
for (const file of files.sort()) {
  if (basename(file) !== 'benchmark.json' || basename(dirname(file)) !== 'new') continue;
  const metadataPath = join(values.criterion, file);
  const metadata = JSON.parse(await readFile(metadataPath, 'utf8'));
  const library = libraries.get(metadata.value_str);
  if (!library || metadata.group_id !== 'PanGUI') continue;
  const estimates = JSON.parse(await readFile(join(dirname(metadataPath), 'estimates.json'), 'utf8'));
  const mean = estimates.mean;
  const confidence = mean.confidence_interval;
  const times = [mean.point_estimate, confidence.lower_bound, confidence.upper_bound];
  if (!times.every(value => Number.isFinite(value) && value > 0) || times[1] > times[0] || times[0] > times[2]) {
    throw new Error(`Invalid estimate in ${metadataPath}`);
  }
  if (typeof metadata.function_id !== 'string' || !metadata.function_id) throw new Error(`Missing scenario in ${metadataPath}`);
  results.push({ scenario: metadata.function_id, library, mean_ns: times[0], lower_ns: times[1], upper_ns: times[2] });
}
const scenarios = new Set(results.filter(result => result.library === 'taffy').map(result => result.scenario));
const templates = (await readdir('benchmarks/pangui')).filter(file => file.endsWith('.txt'));
if (scenarios.size !== templates.length) throw new Error('Missing Taffy scenarios, refusing to publish an incomplete run');
for (const library of libraries.values()) {
  if (!results.some(result => result.library === library)) throw new Error(`No results for ${library}`);
}
const keys = results.map(result => JSON.stringify([result.scenario, result.library]));
if (new Set(keys).size !== keys.length) throw new Error('Duplicate benchmark results');

const metadata = JSON.parse(execFileSync('cargo', [
  'metadata', '--locked', '--format-version', '1', '--manifest-path', 'benchmarks/Cargo.toml', '--features', 'morphorm,torin,yoga,ui-layout,clay,ply,backer,cephal,keru_layout',
], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }));
const versions = Object.fromEntries(metadata.packages
  .filter(packageInfo => [...libraries.values()].includes(packageInfo.name)
    && (packageInfo.name !== 'taffy' || packageInfo.source?.includes(upstream)))
  .map(packageInfo => [packageInfo.name, packageInfo.version]));
for (const library of libraries.values()) {
  if (!versions[library]) throw new Error(`Missing resolved version for ${library}`);
}
const nodeCounts = JSON.parse(execFileSync('cargo', [
  'run', '--quiet', '--locked', '--manifest-path', 'benchmarks/Cargo.toml', '--bin', 'workload_nodes',
], { encoding: 'utf8' }));
for (const scenario of scenarios) {
  if (!Number.isSafeInteger(nodeCounts[scenario]) || nodeCounts[scenario] <= 0) {
    throw new Error(`Missing or invalid node count for ${scenario}`);
  }
}
const date = new Date().toISOString();
const run = {
  id: process.env.GITHUB_RUN_ID || date,
  date,
  commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
  upstream,
  node_counts: nodeCounts,
  versions,
  cpu: cpus()[0]?.model || process.arch,
  runner: process.env.RUNNER_OS || platform(),
  rustc: execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim(),
  url: process.env.GITHUB_RUN_ID && process.env.GITHUB_REPOSITORY
    ? `${process.env.GITHUB_SERVER_URL || 'https://github.com'}/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}`
    : '',
  results,
};
const history = await readHistory(values.history, true);
history.runs = [...history.runs.filter(entry => entry.id !== run.id), run]
  .sort((first, second) => Date.parse(first.date) - Date.parse(second.date));
for (const entry of history.runs) {
  if (entry.upstream !== upstream) continue;
  entry.node_counts ??= {};
  for (const result of entry.results) {
    if (Number.isSafeInteger(nodeCounts[result.scenario])) {
      entry.node_counts[result.scenario] ??= nodeCounts[result.scenario];
    }
  }
}
await writeHistory(values.history, history);
