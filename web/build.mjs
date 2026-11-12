import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, cp } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
const web = path.dirname(fileURLToPath(import.meta.url));
const root = path.dirname(web);
const dist = path.join(web, 'dist');
const { pageMeta } = await import(pathToFileURL(path.join(web, 'src/model.js')));
// The site's path on its host, such as /pencil-puzzles/ for a GitHub Pages project site.
const base = process.env.BASE_PATH || '/';
if (!/^\/([A-Za-z0-9._-]+\/)*$/.test(base)) throw new Error('BASE_PATH must look like / or /name/');
// The site's address without a path, such as https://pencil-puzzles.org. With it, pages get canonical links and link previews, and the build writes a sitemap and robots.txt.
const site = (process.env.SITE_URL || '').replace(/\/+$/, '');
if (site && !/^https?:\/\/[^/]+(\/.*)?$/.test(site)) throw new Error('SITE_URL must look like https://example.org');
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
// Genres that accept only some sizes, such as sudoku, offer those sizes and no custom size. The engine is asked which square sizes it accepts; a gap in them means the sizes are fixed.
function fixedSizes(id) {
  const accepted = [];
  for (let n = 2; n <= 32; n++) if (JSON.parse(engine.request(JSON.stringify({ operation: 'editor', genre: id, shape: `${n}x${n}`, puzzle: null }))).ok) accepted.push(n);
  return accepted.some((n, i) => i > 0 && n !== accepted[i - 1] + 1) ? accepted.map((n) => `${n}x${n}`) : null;
}
const template = (await readFile(path.join(web, 'src/index.html'), 'utf8')).replace('<base href="/">', `<base href="${base}">`);
const attribute = (text) => text.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
const origin = site ? new URL(site).origin : '';
// A page's HTML: the shared shell with the page's own title and description, and its address for search engines and link previews when the site's address is known.
function page(meta, route) {
  const url = `${origin}${base}${route}`;
  const tags = site ? `
  <link rel="canonical" href="${url}">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="pencil-puzzles">
  <meta property="og:title" content="${attribute(meta.title)}">
  <meta property="og:description" content="${attribute(meta.description)}">
  <meta property="og:url" content="${url}">
  <meta property="og:image" content="${origin}${base}social-preview.png">
  <meta name="twitter:card" content="summary_large_image">` : '';
  return template
    .replace('<title>pencil-puzzles</title>', `<title>${attribute(meta.title)}</title>${tags}`)
    .replace(/<meta name="description" content="[^"]*">/, `<meta name="description" content="${attribute(meta.description)}">`);
}
// The seed of each game's example puzzle, where seed 0 does not make a good picture.
const exampleSeeds = { aquarium: '10' };
const routes = [''];
for (const game of catalogue) {
  const id = game.metadata.id;
  if (!/^[a-z][a-z0-9-]*$/.test(id)) throw new Error('Invalid genre route');
  const example = call({ operation: 'generate', genre: id, params: game.params, difficulty: 'easy', seed: exampleSeeds[id] ?? '0' });
  game.example = example.id;
  // The home page shows each game as its solved example board.
  game.picture = call({ operation: 'solve', genre: id, puzzle: example.id }).views[0];
  const fixed = fixedSizes(id);
  game.sizes = fixed || await sizes(id, game.params);
  game.custom = !fixed;
  for (const mode of ['player', 'solver']) {
    const directory = path.join(dist, id, mode);
    await mkdir(directory, { recursive: true });
    await writeFile(path.join(directory, 'index.html'), page(pageMeta(game, mode), `${id}/${mode}/`));
    routes.push(`${id}/${mode}/`);
  }
}
await writeFile(path.join(dist, 'catalogue.json'), JSON.stringify(catalogue));
await writeFile(path.join(dist, 'index.html'), page(pageMeta(), ''));
// Unknown addresses get the shell, which shows the page-not-found message; search engines are told to leave it out.
await writeFile(path.join(dist, '404.html'), template.replace('<title>', '<meta name="robots" content="noindex">\n  <title>'));
await cp(path.join(root, 'docs/images/social-preview.png'), path.join(dist, 'social-preview.png'));
if (site) {
  const urls = routes.map((route) => `  <url><loc>${origin}${base}${route}</loc></url>`);
  await writeFile(path.join(dist, 'sitemap.xml'), ['<?xml version="1.0" encoding="UTF-8"?>', '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">', ...urls, '</urlset>', ''].join('\n'));
  // The documentation build writes its own sitemap under docs/.
  await writeFile(path.join(dist, 'robots.txt'), ['User-agent: *', 'Allow: /', '', `Sitemap: ${origin}${base}sitemap.xml`, `Sitemap: ${origin}${base}docs/sitemap.xml`, ''].join('\n'));
}
console.log(`Built ${catalogue.length} games in ${dist}`);
