export function route(pathname, catalogue) {
  const parts = pathname.split('/').filter(Boolean);
  if (!parts.length) return {page:'home'};
  const game = catalogue.find(g => g.metadata.id === parts[0]);
  return game && parts.length === 2 && ['player','solver'].includes(parts[1]) ? {page:parts[1],game} : {page:'missing'};
}
/** The title and description of a page, for the browser tab, search results and link previews. Without a game it is the home page. */
export function pageMeta(game, page) {
  if (!game) return {title:'pencil-puzzles: play and solve pencil puzzles online',description:'Play sudoku, kakuro, nurikabe and more in your browser. Every puzzle has one solution and a level from easy to expert.'};
  const name = game.metadata.name, lower = name.toLowerCase();
  return page === 'solver'
    ? {title:`${name} solver · pencil-puzzles`,description:`Enter ${lower} clues or paste a puzzle ID to get the solution.`}
    : {title:`${name}: play online · pencil-puzzles`,description:`Play ${lower} online from easy to expert. ${game.metadata.summary}`};
}
const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
/** Undoable cell values plus per-cell notes; notes default to empty strings. */
export class History {
  constructor(values, notes = values.map(() => '')) { this.values = [...values]; this.notes = [...notes]; this.past = []; this.future = []; }
  set(values, notes = this.notes) {
    if (same(values, this.values) && same(notes, this.notes)) return false;
    this.past.push([this.values, this.notes]); this.values = [...values]; this.notes = [...notes]; this.future = [];
    return true;
  }
  undo() { if (!this.past.length) return false; this.future.push([this.values, this.notes]); [this.values, this.notes] = this.past.pop(); return true; }
  redo() { if (!this.future.length) return false; this.past.push([this.values, this.notes]); [this.values, this.notes] = this.future.pop(); return true; }
  /** The most recent `limit` undo and redo steps, for saving. */
  steps(limit = 200) { return {past: this.past.slice(-limit), future: this.future.slice(-limit)}; }
  /** Restore saved steps; steps that fail `valid([values, notes])` drop the whole history. */
  restore({past, future}, valid) {
    const ok = list => Array.isArray(list) && list.every(step => Array.isArray(step) && step.length === 2 && valid(step));
    if (ok(past) && ok(future)) { this.past = past.map(([v, n]) => [[...v], [...n]]); this.future = future.map(([v, n]) => [[...v], [...n]]); }
    return this;
  }
}
/** Notes are comma-separated mark values kept in mark order. */
export function toggleNote(notes, value, marks) {
  const set = new Set(notes ? notes.split(',') : []);
  set.has(value) ? set.delete(value) : set.add(value);
  return marks.map(m => m.value).filter(v => set.has(v)).join(',');
}
/** Notes without `value`. */
export function removeNote(notes, value) { return notes ? notes.split(',').filter(v => v !== value).join(',') : notes; }
export function formatTime(ms) {
  const s = Math.floor(ms / 1000), h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60, pad = n => String(n).padStart(2, '0');
  return h ? `${h}:${pad(m)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`;
}
/** Sets of `length` distinct digits from 1 to `max` that add up to `sum` and contain every digit in `required`, smallest first. */
export function combinations(sum, length, max, required = []) {
  const out = [], pick = [];
  const walk = (from, left) => {
    if (pick.length === length) { if (left === 0 && required.every(d => pick.includes(d))) out.push([...pick]); return; }
    for (let d = from; d <= max && d <= left; d++) { pick.push(d); walk(d + 1, left - d); pick.pop(); }
  };
  walk(1, sum);
  return out;
}
/** Keep `id` first in a list of recent puzzle IDs; returns the new list and the IDs that fell off the end. */
export function remember(list, id, limit) {
  const next = [id, ...(Array.isArray(list) ? list : []).filter(x => x !== id && typeof x === 'string')];
  return {list: next.slice(0, limit), dropped: next.slice(limit)};
}
