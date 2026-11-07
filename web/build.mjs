import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, cp } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
const web = path.dirname(fileURLToPath(import.meta.url));
const root = path.dirname(web);
const dist = path.join(web, 'dist');
// The site's path on its host, such as /pencil-puzzles/ for a GitHub Pages project site.
const base = process.env.BASE_PATH || '/';
if (!/^\/([A-Za-z0-9._-]+\/)*$/.test(base)) throw new Error('BASE_PATH must look like / or /name/');
const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: 'inherit' });
run('cargo', ['build', '-p', 'pencil-puzzles-web', '--release', '--target', 'wasm32-unknown-unknown', '--locked']);
await mkdir(path.join(dist, 'wasm'), { recursive: true });
run('wasm-bindgen', ['--target', 'web', '--no-typescript', '--out-dir', path.join(dist, 'wasm'), path.join(root, 'target/wasm32-unknown-unknown/release/pencil_puzzles_web.wasm')]);
await cp(path.join(web, 'src'), dist, { recursive: true });
const engine = await import(pathToFileURL(path.join(dist, 'wasm/pencil_puzzles_web.js')));
await engine.default({ module_or_path: await readFile(path.join(dist, 'wasm/pencil_puzzles_web_bg.wasm')) });
const call = (data) => {
  const result = JSON.parse(engine.request(JSON.stringify(data)));
  if (!result.ok) throw new Error(result.error);
  return result.value;
};
const catalogue = call({ operation: 'list' });
// Size presets are the shapes in the genre's benchmark table, plus its default size.
async function sizes(id, params) {
  let shapes = [];
  try { shapes = (await readFile(path.join(root, 'docs/benchmarks', `${id}.csv`), 'utf8')).trim().split(/\r?\n/).slice(1).map((row) => row.split(',')[0]); } catch { /* A genre without benchmarks gets its default size only. */ }
  const dims = (s) => s.split('x').map(Number);
  return [...new Set([...shapes, params])].filter((s) => /^\d+x\d+$/.test(s)).sort((a, b) => dims(a)[0] - dims(b)[0] || dims(a)[1] - dims(b)[1]);
}
const html = (await readFile(path.join(web, 'src/index.html'), 'utf8')).replace('<base href="/">', `<base href="${base}">`);
for (const game of catalogue) {
  const id = game.metadata.id;
  if (!/^[a-z][a-z0-9-]*$/.test(id)) throw new Error('Invalid genre route');
  const example = call({ operation: 'generate', genre: id, params: game.params, difficulty: 'easy', seed: '0' });
  game.example = example.id;
  game.sizes = await sizes(id, game.params);
  for (const page of ['player', 'solver']) {
    const directory = path.join(dist, id, page);
    await mkdir(directory, { recursive: true });
    await writeFile(path.join(directory, 'index.html'), html.replace('<title>pencil-puzzles</title>', `<title>${game.metadata.name} ${page} · pencil-puzzles</title>`));
  }
}
await writeFile(path.join(dist, 'catalogue.json'), JSON.stringify(catalogue));
await writeFile(path.join(dist, 'index.html'), html);
await writeFile(path.join(dist, '404.html'), html);
console.log(`Built ${catalogue.length} games in ${dist}`);
