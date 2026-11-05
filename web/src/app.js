import {route,History,toggleNote,formatTime} from './model.js';
const $ = (selector,root=document) => root.querySelector(selector);
const main = $('#main');
const escape = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const element = (tag,text,className) => {const e=document.createElement(tag); if(text!==undefined)e.textContent=text;if(className)e.className=className;return e;};
const link = (text,href,current) => {const a=element('a',text);a.href=href;if(current)a.setAttribute('aria-current','page');return a;};
const readSaved = key => {try{return JSON.parse(localStorage.getItem(key));}catch{return null;}};
const save = (key,value) => {try{localStorage.setItem(key,JSON.stringify(value));}catch{/* Playing remains available without browser storage. */}};
const icons = {
  undo:'<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 7h6a3 3 0 0 1 0 6H8M6.5 4.5 4 7l2.5 2.5"/></svg>',
  redo:'<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M12 7H6a3 3 0 0 0 0 6h2M9.5 4.5 12 7l-2.5 2.5"/></svg>',
};

// Theme follows the system until the visitor picks one; picking the system's own theme returns to following it.
const themeButton=$('#theme'),systemDark=matchMedia('(prefers-color-scheme: dark)');
const storedTheme=()=>{try{return localStorage.getItem('pencil-theme');}catch{return null;}};
function themeLabel(){const label=document.documentElement.dataset.theme==='dark'?'Switch to light mode':'Switch to dark mode';themeButton.setAttribute('aria-label',label);themeButton.title=label;}
themeLabel();
themeButton.onclick=()=>{const theme=document.documentElement.dataset.theme==='dark'?'light':'dark';document.documentElement.dataset.theme=theme;try{if(theme===(systemDark.matches?'dark':'light'))localStorage.removeItem('pencil-theme');else localStorage.setItem('pencil-theme',theme);}catch{}themeLabel();};
systemDark.addEventListener('change',()=>{if(!storedTheme()){document.documentElement.dataset.theme=systemDark.matches?'dark':'light';themeLabel();}});

class Engine {
  constructor(){this.worker=null;this.pending=new Map();this.next=0;}
  call(request){
    if(!this.worker){this.worker=new Worker(new URL('worker.js',document.baseURI),{type:'module'});this.worker.onmessage=({data})=>{const p=this.pending.get(data.id);if(!p)return;clearTimeout(p.timer);this.pending.delete(data.id);data.ok?p.resolve(data.value):p.reject(new Error(data.error));};this.worker.onerror=()=>this.stop('The puzzle engine could not start. Reload the page and try again.');}
    return new Promise((resolve,reject)=>{const id=++this.next;const timer=setTimeout(()=>this.stop('This puzzle took too long. Try a smaller size or another puzzle.'),15000);this.pending.set(id,{resolve,reject,timer});this.worker.postMessage({id,request});});
  }
  stop(message){this.worker?.terminate();this.worker=null;for(const p of this.pending.values()){clearTimeout(p.timer);p.reject(new Error(message));}this.pending.clear();}
}
const engine=new Engine();
function errorMessage(error){return /budget/i.test(error.message)?'This puzzle needs more time. Try a smaller size, a lower difficulty, or another puzzle.':error.message;}
function board(view,{interactive=false,notes,selected=-1}={}){
  const table=element('table',undefined,'board');table.setAttribute('aria-label',interactive?'Puzzle board':'Puzzle solution');
  const left=view.left.length>0, right=view.right.length>0, blank=view.marks[0]?.value, focus=selected>=0?selected:view.cells.findIndex(cell=>cell.editable);
  table.style.setProperty('--cols',String(view.cols+(left?1:0)+(right?1:0)));
  const entry=view.marks.length>3, from=entry&&selected>=0?view.cells[selected]:null;
  const same=from&&from.value!==blank?from.value:null;
  const sr=Math.floor(selected/view.cols), sc=selected%view.cols;
  if(view.top.length){const row=document.createElement('tr');if(left)row.append(element('th',''));for(let c=0;c<view.cols;c++){const th=element('th',view.top[c]||'');th.scope='col';row.append(th);}if(right)row.append(element('th',''));table.append(row);}
  for(let r=0;r<view.rows;r++){
    const row=document.createElement('tr');if(left){const th=element('th',view.left[r]||'');th.scope='row';row.append(th);}
    for(let c=0;c<view.cols;c++){
      const i=r*view.cols+c,cell=view.cells[i],choice=view.marks.find(m=>m.value===cell.value),noted=!!notes?.[i]&&cell.value===blank;
      const td=document.createElement('td');td.dataset.cellRow=String(r);td.dataset.cellCol=String(c);if(cell.region!==null&&cell.region!==undefined)td.dataset.region=String(cell.region);
      if(view.row_separators?.includes(r))td.classList.add('box-top');if(view.col_separators?.includes(c))td.classList.add('box-left');
      if(r===0)td.classList.add('edge-top');if(r===view.rows-1)td.classList.add('edge-bottom');if(c===0)td.classList.add('edge-left');if(c===view.cols-1)td.classList.add('edge-right');
      const peer=from&&i!==selected&&(r===sr||c===sc||(cell.region!=null&&cell.region===from.region));
      const button=element(interactive?'button':'span',choice?.symbol??cell.value,'cell'+(choice?.shaded?' shaded':'')+(choice?.symbol==='·'?' dot':'')+(!cell.editable?' given':'')+(i===selected?' selected':'')+(same!==null&&cell.value===same?' same':'')+(peer?' peer':''));button.dataset.index=String(i);
      if(noted){const set=new Set(notes[i].split(',')),grid=element('span',undefined,'notes');grid.style.setProperty('--note-cols',String(Math.ceil(Math.sqrt(view.marks.length-1))));for(const m of view.marks.slice(1))grid.append(element('span',set.has(m.value)?m.symbol:''));button.replaceChildren(grid);}
      button.setAttribute('aria-label',`Row ${r+1}, column ${c+1}: ${noted?'notes '+notes[i]:choice?.name??cell.value}`);
      if(interactive){button.type='button';button.tabIndex=i===focus?0:-1;}
      td.append(button);row.append(td);
    }
    if(right){const clue=element('th',view.right[r]||'','clue');clue.dataset.row=String(r);row.append(clue);}table.append(row);
  }
  if(view.bottom.length){const row=document.createElement('tr');if(left)row.append(element('th',''));for(let c=0;c<view.cols;c++){const clue=element('th',view.bottom[c]||'','clue');clue.dataset.col=String(c);row.append(clue);}if(right)row.append(element('th',''));table.append(row);}
  return table;
}
function directory(catalogue,current) {
  const nav=$('#directory');
  nav.replaceChildren(link('Overview','./',current.page==='home'),element('p','Games','label'));
  for(const game of catalogue){
    const id=game.metadata.id,open=current.game===game,wrap=element('div',undefined,'game'+(open?' open':''));
    wrap.append(link(game.metadata.name,`${id}/${current.page==='solver'?'solver':'player'}/`,false));
    if(open){const modes=element('div',undefined,'modes');modes.append(link('Player',`${id}/player/`,current.page==='player'),link('Solver',`${id}/solver/`,current.page==='solver'));wrap.append(modes);}
    nav.append(wrap);
  }
  if(current.game){const sep=()=>element('span','/','sep'),here=element('span',current.page==='solver'?'Solver':'Player');here.setAttribute('aria-current','page');$('#trail').append(sep(),link(current.game.metadata.name,`${current.game.metadata.id}/player/`,false),sep(),here);}
  const toggle=$('#show-games'),isOpen=()=>document.body.classList.contains('directory-open');
  const setOpen=open=>{document.body.classList.toggle('directory-open',open);toggle.setAttribute('aria-expanded',String(open));};
  toggle.onclick=()=>setOpen(!isOpen());
  document.addEventListener('click',e=>{if(isOpen()&&!e.target.closest('#sidebar,#show-games'))setOpen(false);});
  document.addEventListener('keydown',e=>{if(e.key==='Escape'&&isOpen()){setOpen(false);toggle.focus();}});
}
function home(catalogue){
  document.title='pencil-puzzles';
  const repo='https://github.com/emieks/pencil-puzzles';
  main.innerHTML=`<h1>pencil-puzzles</h1>
<div class="prose"><p>Open-source generators, solvers and difficulty raters for pencil puzzles, written in Rust. Every generated puzzle has exactly one solution and a rating on a shared difficulty scale.</p>
<p>This site runs the same engine in your browser through WebAssembly, so every supported game can be played and solved here. The library is also available as a command-line tool and a Python package.</p></div>
<h2>Games</h2><table class="overview"><thead><tr><th>Game</th><th class="description">Goal</th><th class="status-cell">Status</th><th>Open</th></tr></thead><tbody id="games"></tbody></table>
<h2>Contribute</h2>
<div class="prose"><p>pencil-puzzles is MIT-licensed, and contributions of every size are welcome. Report a bug or a rating that feels off, make generation faster for large grids, or add a new genre.</p>
<p class="link-row"><a href="${repo}" target="_blank" rel="noopener noreferrer">Source on GitHub</a><a href="${repo}/blob/main/CONTRIBUTING.md" target="_blank" rel="noopener noreferrer">Contributing guide</a><a href="${repo}/issues" target="_blank" rel="noopener noreferrer">Open an issue</a></p></div>`;
  for(const game of catalogue){const m=game.metadata;const row=document.createElement('tr');row.innerHTML=`<td class="game-name"><a href="${m.id}/player/">${escape(m.name)}</a></td><td class="description">${escape(m.summary)}</td><td class="status-cell">${escape(m.status)}</td><td class="tools"><a href="${m.id}/player/">Player</a><a href="${m.id}/solver/">Solver</a></td>`;$('#games').append(row);}
}
function heading(game,page){const id=game.metadata.id;main.innerHTML=`<div class="title-row"><h1>${escape(game.metadata.name)}</h1><nav class="mode-links" aria-label="Game mode"><a id="tab-player" href="${id}/player/" ${page==='player'?'aria-current="page"':''}>Player</a><a id="tab-solver" href="${id}/solver/" ${page==='solver'?'aria-current="page"':''}>Solver</a></nav></div>`;document.title=`${game.metadata.name} ${page} · pencil-puzzles`;}
function status(message,kind=''){const box=$('#status');box.className='status'+(kind?' '+kind:'');box.textContent=message;}
function keyList(rows){return rows.flatMap(([keys,action])=>{const dt=element('dt');keys.split(' / ').forEach((k,i)=>{if(i)dt.append(' ');dt.append(element('kbd',k));});return [dt,element('dd',action)];});}
function player(game){
  heading(game,'player');
  const id=game.metadata.id;
  main.insertAdjacentHTML('beforeend',`<section aria-label="Puzzle player"><fieldset id="controls" class="player-layout">
<div class="toolbar"><div class="field"><label for="params">Size</label><input id="params" value="${escape(game.params)}" spellcheck="false" autocomplete="off"></div><div class="field"><label for="difficulty">Difficulty</label><select id="difficulty"><option value="easy">Easy</option><option value="medium">Medium</option><option value="hard">Hard</option><option value="expert">Expert</option></select></div><button id="new-game" type="button">New puzzle</button></div>
<div class="board-block"><div class="board-area"><div id="board"></div></div>
<div class="board-caption"><span class="stats"><span id="timer">0:00</span> · <span id="progress"></span></span><button id="share" class="link-button" type="button">Copy link</button></div></div>
<div class="side">
<div id="legend" class="legend"></div>
<p id="finish-note" class="finish-note"></p>
<div class="board-tools"><span class="group"><button id="undo" class="square" type="button" aria-label="Undo" title="Undo (Ctrl+Z)">${icons.undo}</button><button id="redo" class="square" type="button" aria-label="Redo" title="Redo (Ctrl+Y)">${icons.redo}</button></span><button id="clear" type="button" title="Clear all marks (undo restores them)">Clear</button><button id="check" class="primary push" type="button">Check</button></div>
<p id="status" class="status" role="status" aria-live="polite">Loading…</p>
<dl id="keys" class="keys"></dl>
</div></fieldset></section>
<details id="rules-box"><summary>Rules</summary><p id="rules"></p></details><details><summary>Load puzzle</summary><label for="load-input">ID, text, or JSON</label><textarea id="load-input" maxlength="1000000" spellcheck="false" placeholder="${escape(game.example)}"></textarea><button id="load" type="button">Load</button></details>`);
  const key='pencil-game:'+id;let current=null;let history=null;let busy=false;let entry=false;let selected=-1;let notesMode=false;let drag=null;let solvedTime=null;let checkTimer=0;let checkSeq=0;
  const clock={elapsed:0,since:null};const boardBox=$('#board');
  const blank=()=>current.view.marks[0].value;
  // Shading genres have one shaded mark and one dot mark; marking a cell with the mark it already has clears it.
  const shadeMark=()=>current.view.marks.find(m=>m.shaded)?.value,dotMark=()=>current.view.marks.find((m,i)=>i>0&&!m.shaded)?.value;
  const toggled=(i,value)=>history.values[i]===value?blank():value;const editable=i=>i>=0&&current.view.cells[i].editable;
  const elapsed=()=>clock.elapsed+(clock.since===null?0:performance.now()-clock.since);
  function startClock(){if(current&&clock.since===null&&solvedTime===null&&!document.hidden)clock.since=performance.now();}
  function pauseClock(){if(clock.since!==null){clock.elapsed+=performance.now()-clock.since;clock.since=null;}}
  const tick=()=>{const t=$('#timer');t.textContent=(solvedTime===null?'':'Solved · ')+formatTime(solvedTime??elapsed());t.classList.toggle('solved',solvedTime!==null);};setInterval(tick,1000);
  document.addEventListener('visibilitychange',()=>{if(document.hidden){pauseClock();persist();}else startClock();});addEventListener('pagehide',()=>persist());
  function sync(){const ready=!!current&&!busy;$('#undo').disabled=!ready||!history?.past.length;$('#redo').disabled=!ready||!history?.future.length;for(const b of ['check','clear','share'])$('#'+b).disabled=!ready;$('#load').disabled=busy;$('#load-input').disabled=busy;}
  function persist(){if(current)save(key,{id:current.id,marks:history.values,notes:history.notes,elapsed:elapsed(),solved:solvedTime,params:$('#params').value,difficulty:$('#difficulty').value});}
  function draw(values=history.values,focus=boardBox.contains(document.activeElement)){
    const view={...current.view,cells:current.view.cells.map((cell,i)=>({...cell,value:values[i]}))};
    const table=board(view,{interactive:true,notes:history.notes,selected});table.classList.add(entry?'entry':'paint');boardBox.replaceChildren(table);
    if(focus&&selected>=0)boardBox.querySelector(`[data-index="${selected}"]`)?.focus({preventScroll:true});
    $('#progress').textContent=`${values.filter((v,i)=>view.cells[i].editable&&v!==current.view.cells[i].value).length} / ${view.cells.filter(c=>c.editable).length} cells`;
    sync();
  }
  function changed(){draw();persist();status('');scheduleCheck();}
  function commit(values,notes=history.notes){if(history.set(values,notes))changed();}
  function apply(i,value){if(!editable(i))return;const values=[...history.values];values[i]=value;commit(values);}
  function enter(value,asNote){
    if(!editable(selected))return;const values=[...history.values],notes=[...history.notes];
    if(value===blank()){if(values[selected]!==blank())values[selected]=blank();else notes[selected]='';}
    else if(asNote){if(values[selected]!==blank())return;notes[selected]=toggleNote(notes[selected],value,current.view.marks);}
    else values[selected]=value;
    commit(values,notes);
  }
  function undo(){if(history.undo())changed();}
  function redo(){if(history.redo())changed();}
  function solved(){if(solvedTime===null){pauseClock();solvedTime=elapsed();tick();persist();}status(`Solved in ${formatTime(solvedTime)}.`,'success');}
  function scheduleCheck(){clearTimeout(checkTimer);checkTimer=setTimeout(async()=>{const seq=++checkSeq,marks=history.values;if(entry&&marks.some((v,i)=>editable(i)&&v===blank()))return;try{const result=await engine.call({operation:'check',genre:id,puzzle:current.id,marks,finished:true});if(seq===checkSeq&&result.solved)solved();}catch{/* The Check button reports errors. */}},200);}
  function setNotes(on){notesMode=on;$('#notes')?.setAttribute('aria-pressed',String(on));}
  boardBox.addEventListener('contextmenu',e=>{if(e.target.closest('.cell'))e.preventDefault();});
  boardBox.addEventListener('pointerdown',e=>{
    const cell=e.target.closest('.cell');if(!cell||busy||!current||(e.button!==0&&e.button!==2))return;
    e.preventDefault();selected=Number(cell.dataset.index);
    if(entry||!editable(selected)){draw(undefined,true);return;}
    const target=e.button===2?dotMark():shadeMark();if(target===undefined){draw(undefined,true);return;}
    drag={value:toggled(selected,target),values:[...history.values]};drag.values[selected]=drag.value;draw(drag.values,true);
  });
  addEventListener('pointermove',e=>{if(!drag)return;const cell=document.elementFromPoint(e.clientX,e.clientY)?.closest?.('#board .cell');if(!cell)return;const i=Number(cell.dataset.index);if(editable(i)&&drag.values[i]!==drag.value){drag.values[i]=drag.value;draw(drag.values);}});
  const endDrag=()=>{if(!drag)return;const values=drag.values;drag=null;commit(values);};addEventListener('pointerup',endDrag);addEventListener('pointercancel',endDrag);
  document.addEventListener('keydown',e=>{
    if(!current||busy||e.target.closest?.('input,textarea,select'))return;
    const mod=e.ctrlKey||e.metaKey,k=e.key.toLowerCase(),view=current.view,inBoard=boardBox.contains(e.target);
    if(mod&&(k==='z'||k==='y')){e.preventDefault();k==='y'||e.shiftKey?redo():undo();return;}
    if(mod||e.altKey)return;
    if(entry&&k==='n'){setNotes(!notesMode);return;}
    if(!inBoard&&!(selected>=0&&(e.target===document.body||e.target.closest?.('#legend'))))return;
    if(inBoard&&e.target.dataset.index)selected=Number(e.target.dataset.index);
    const moves={ArrowLeft:[0,-1],ArrowRight:[0,1],ArrowUp:[-1,0],ArrowDown:[1,0],a:[0,-1],d:[0,1],w:[-1,0],s:[1,0]};
    const move=moves[e.key.length===1?k:e.key];
    if(move){e.preventDefault();const r=Math.floor(selected/view.cols)+move[0],c=selected%view.cols+move[1];if(r>=0&&r<view.rows&&c>=0&&c<view.cols)selected=r*view.cols+c;draw(undefined,true);return;}
    const digit=/^(?:Digit|Numpad)(\d)$/.exec(e.code)?.[1];
    if(digit!==undefined||e.key==='Backspace'||e.key==='Delete'){const mark=view.marks[digit===undefined?0:Number(digit)];if(!mark)return;e.preventDefault();entry?enter(mark.value,notesMode||e.shiftKey):apply(selected,mark.value);return;}
    const keyMark=entry?undefined:{q:shadeMark(),e:dotMark(),r:blank()}[k];
    if(keyMark!==undefined&&selected>=0){e.preventDefault();apply(selected,k==='r'?keyMark:toggled(selected,keyMark));}
  });
  async function action(message,task){if(busy)return;busy=true;$('#controls').disabled=true;sync();status(message);try{await task();}catch(e){status(errorMessage(e),'error');}finally{busy=false;$('#controls').disabled=false;sync();}}
  function accept(value,restore){
    pauseClock();checkSeq++;current=value;entry=value.view.marks.length>3;selected=-1;drag=null;
    const initial=value.view.cells.map(c=>c.value);const allowed=new Set(value.view.marks.map(m=>m.value));
    const saved=restore&&readSaved(key);const valid=saved?.id===value.id&&Array.isArray(saved.marks)&&saved.marks.length===initial.length&&saved.marks.every((m,i)=>value.view.cells[i].editable?allowed.has(m):m===initial[i]);
    const notes=valid&&Array.isArray(saved.notes)&&saved.notes.length===initial.length&&saved.notes.every(n=>typeof n==='string')?saved.notes:initial.map(()=>'');
    history=new History((valid?saved.marks:initial).map((v,i)=>value.view.cells[i].editable?v:initial[i]),notes);
    clock.elapsed=valid&&Number.isFinite(saved.elapsed)?saved.elapsed:0;clock.since=null;solvedTime=valid&&Number.isFinite(saved.solved)?saved.solved:null;
    if(value.params)$('#params').value=value.params;$('#rules').textContent=value.view.instructions;
    $('#finish-note').textContent=value.view.finish_note||'';
    const marks=value.view.marks;
    $('#keys').replaceChildren(...keyList(entry
      ?[[`${marks[1].symbol}–${marks.at(-1).symbol}`,'enter'],['Shift+digit','note'],['N','notes mode'],['Del','erase'],['Arrows / WASD','move'],['Ctrl+Z / Ctrl+Y','undo / redo']]
      :[['Click / Q',marks.find(m=>m.shaded)?.name.toLowerCase()],['Right-click / E',marks.find((m,i)=>i>0&&!m.shaded)?.name.toLowerCase()],['R / Del','clear'],['Drag','mark several'],['Arrows / WASD','move'],['Ctrl+Z / Ctrl+Y','undo / redo']]));
    const legendButton=(text,label,click)=>{const button=element('button',text);button.type='button';button.setAttribute('aria-label',label);button.onclick=click;button.onmousedown=e=>e.preventDefault();return button;};
    $('#legend').className='legend'+(entry?' digits':'');
    if(entry){
      const notesButton=legendButton('Notes','Notes mode',()=>setNotes(!notesMode));notesButton.id='notes';
      const pad=element('div',undefined,'keypad');pad.style.setProperty('--keypad-cols',String(Math.ceil(Math.sqrt(marks.length-1))));pad.append(...marks.slice(1).map(m=>legendButton(m.symbol,`Enter ${m.name}`,()=>enter(m.value,notesMode))));
      const tools=element('div',undefined,'keypad-tools');tools.append(notesButton,legendButton('Erase','Erase cell',()=>enter(blank(),false)));
      $('#legend').replaceChildren(pad,tools);
    }
    else $('#legend').replaceChildren(...marks.map(m=>{const label=element('span');const swatch=element('span',m.symbol,'swatch'+(m.shaded?' shaded':''));swatch.setAttribute('aria-hidden','true');label.append(swatch,document.createTextNode(m.name));return label;}));
    setNotes(false);
    $('#tab-solver').href=`${id}/solver/?puzzle=${encodeURIComponent(value.id)}`;    window.history.replaceState(null,'',location.pathname+`?puzzle=${encodeURIComponent(value.id)}`);
    draw();startClock();tick();persist();if(solvedTime===null)status('');else solved();
  }
  async function newGame(){await action('Loading…',async()=>{const bytes=new Uint32Array(2);crypto.getRandomValues(bytes);const seed=((BigInt(bytes[0])<<32n)|BigInt(bytes[1])).toString();accept(await engine.call({operation:'generate',genre:id,params:$('#params').value,difficulty:$('#difficulty').value,seed}),false);});}
  $('#new-game').onclick=newGame;
  $('#params').addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();newGame();}});
  $('#undo').onclick=undo;$('#redo').onclick=redo;
  $('#clear').onclick=()=>{commit(current.view.cells.map(c=>c.value),current.view.cells.map(()=>''));status('Cleared. Undo restores it.');};
  $('#check').onclick=()=>action('Checking…',async()=>{const result=await engine.call({operation:'check',genre:id,puzzle:current.id,marks:history.values,finished:true});$('#board').querySelectorAll('.invalid').forEach(c=>c.classList.remove('invalid'));for(const v of result.violations){const p=v.place;let selector=p==='board'?'td':'';if(p&&typeof p==='object'){if('row'in p)selector=`[data-row="${p.row}"],[data-cell-row="${p.row}"]`;else if('col'in p)selector=`[data-col="${p.col}"],[data-cell-col="${p.col}"]`;else if('region'in p)selector=`[data-region="${p.region}"]`;else if('cell'in p)selector=`[data-index="${p.cell.row*current.view.cols+p.cell.col}"]`;}if(selector)$('#board').querySelectorAll(selector).forEach(c=>c.classList.add('invalid'));}if(result.solved)solved();else status('Not solved. Check the marked cells or clues.','error');});
  $('#load').onclick=()=>action('Loading…',async()=>accept(await engine.call({operation:'load',genre:id,puzzle:$('#load-input').value}),false));
  let shareTimer=0;
  $('#share').onclick=async()=>{const button=$('#share');try{await navigator.clipboard.writeText(location.href);button.textContent='Copied';clearTimeout(shareTimer);shareTimer=setTimeout(()=>{button.textContent='Copy link';},1500);}catch{status('Copy the URL from the address bar.');}};
  const rulesBox=$('#rules-box'),rulesKey='pencil-rules:'+id,rulesOpen=readSaved(rulesKey);rulesBox.open=rulesOpen===null?true:!!rulesOpen;rulesBox.ontoggle=()=>save(rulesKey,rulesBox.open);
  const saved=readSaved(key);if(saved?.params)$('#params').value=saved.params;if(saved?.difficulty&&[...$('#difficulty').options].some(o=>o.value===saved.difficulty))$('#difficulty').value=saved.difficulty;
  const supplied=new URLSearchParams(location.search).get('puzzle')||saved?.id;
  if(supplied)action('Loading…',async()=>accept(await engine.call({operation:'load',genre:id,puzzle:supplied}),true));else newGame();
  sync();
}
function solver(game){
  heading(game,'solver');
  const id=game.metadata.id;
  main.insertAdjacentHTML('beforeend',`<section aria-label="Puzzle solver"><fieldset id="solver-controls" class="solver-layout">
<div class="toolbar"><div class="field"><label for="editor-shape">Size</label><input id="editor-shape" value="${escape(game.params)}" spellcheck="false" autocomplete="off"></div><button id="new-grid" type="button">New grid</button><button id="example" type="button">Example</button></div>
<div id="editor-grid" class="board-area"></div>
<div class="actions"><button id="clear-clues" type="button">Clear clues</button><button id="solve" class="primary" type="button">Solve</button></div>
<p id="status" class="status" role="status" aria-live="polite"></p>
<div class="result-head result-part"><h2 id="result-title"></h2><a id="play-link" href="${id}/player/">Play this puzzle</a></div>
<div id="solutions" class="result-boards result-part" aria-label="Solver result"></div>
<p id="result-id" class="puzzle-id result-part"></p>
</fieldset></section>
<details><summary>Import ID or text</summary><label for="puzzle-input">ID, text, or JSON</label><textarea id="puzzle-input" maxlength="1000000" spellcheck="false" placeholder="${escape(game.example)}"></textarea><button id="import" type="button">Import</button></details>`);
  let editor=null;let busy=false;let fields=[];
  const showResult=on=>{$('#solver-controls').classList.toggle('has-result',on);if(!on)$('#tab-player').href=`${id}/player/`;};
  function invalidate(){showResult(false);status('');}
  function label(place){if('row'in place)return `Row ${place.row+1} clue`;if('col'in place)return `Column ${place.col+1} clue`;return `Row ${place.cell.row+1}, column ${place.cell.col+1}`;}
  function drawEditor(value){
    editor=value;$('#editor-shape').value=value.shape;
    const table=board(value.view);table.setAttribute('aria-label','Puzzle clues');
    fields=value.fields.map((field,i)=>{
      const input=document.createElement('input');input.type='number';input.inputMode='numeric';input.min=String(field.min);input.max=String(field.max);input.step='1';input.required=!field.optional;input.value=field.value;input.className='clue-input';input.setAttribute('aria-label',label(field.place));input.dataset.field=String(i);input.oninput=invalidate;
      let target;
      if('row'in field.place)target=table.querySelector(`[data-row="${field.place.row}"]`);
      else if('col'in field.place)target=table.querySelector(`[data-col="${field.place.col}"]`);
      else {const cell=table.querySelector(`[data-index="${field.place.cell.row*value.view.cols+field.place.cell.col}"]`);target=cell?.parentElement;}
      if(!target)throw new Error('This puzzle editor has an unsupported clue location.');target.replaceChildren(input);return input;
    });
    $('#editor-grid').replaceChildren(table);invalidate();
  }
  async function action(message,run){if(busy)return;busy=true;$('#solver-controls').disabled=true;$('#import').disabled=true;status(message);try{await run();}catch(e){status(errorMessage(e),'error');}finally{busy=false;$('#solver-controls').disabled=false;$('#import').disabled=false;}}
  async function open(puzzle){await action('Loading…',async()=>{drawEditor(await engine.call({operation:'editor',genre:id,shape:$('#editor-shape').value,puzzle:puzzle||null}));status('');});}
  $('#new-grid').onclick=()=>open(null);$('#example').onclick=()=>open(game.example);
  $('#editor-shape').addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();open(null);}});
  $('#clear-clues').onclick=()=>{for(const input of fields)input.value='';invalidate();fields[0]?.focus();};
  $('#import').onclick=()=>open($('#puzzle-input').value);
  $('#solve').onclick=async()=>{
    if(!editor||busy)return;
    for(const input of fields)if(!input.reportValidity())return;
    await action('Solving…',async()=>{
      showResult(false);
      const puzzle=await engine.call({operation:'compose',genre:id,shape:editor.shape,values:fields.map(input=>input.value)});
      const result=await engine.call({operation:'solve',genre:id,puzzle:puzzle.id});
      status({unique:'Unique solution.',multiple:'Multiple solutions. Two are shown.',none:'No solution. Check the clues.'}[result.status],result.status==='none'?'error':'success');
      if(result.views.length){$('#result-title').textContent=result.status==='unique'?'Solution':'Two solutions';$('#solutions').replaceChildren(...result.views.map((v,i)=>{const wrap=element('div',undefined,'result-board');if(result.views.length>1)wrap.append(element('p',`Answer ${i+1}`,'small'));wrap.append(board(v));return wrap;}));$('#result-id').textContent=result.puzzle.id;const play=`${id}/player/?puzzle=${encodeURIComponent(result.puzzle.id)}`;$('#play-link').href=play;$('#tab-player').href=play;showResult(true);}
    });
  };
  open(new URLSearchParams(location.search).get('puzzle'));
}

try{
  const response=await fetch('catalogue.json');if(!response.ok)throw new Error('Could not load the game collection.');const catalogue=await response.json();
  const current=route(location.pathname.slice(new URL(document.baseURI).pathname.length-1),catalogue);directory(catalogue,current);if(current.page==='home')home(catalogue);else if(current.page==='player')player(current.game);else if(current.page==='solver')solver(current.game);else{document.title='Page not found · pencil-puzzles';main.innerHTML='<h1>Page not found</h1><p>That game or page is not in this collection.</p><p><a href="./">Back to all games</a></p>';}
}catch(e){main.replaceChildren(element('h1','Unable to open the collection'),element('p',errorMessage(e)));}
