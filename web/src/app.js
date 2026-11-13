import {route,pageMeta,History,toggleNote,removeNote,formatTime,combinations,remember} from './model.js';
const $ = (selector,root=document) => root.querySelector(selector);
const main = $('#main');
const escape = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const element = (tag,text,className) => {const e=document.createElement(tag); if(text!==undefined)e.textContent=text;if(className)e.className=className;return e;};
const link = (text,href,current) => {const a=element('a',text);a.href=href;if(current)a.setAttribute('aria-current','page');return a;};
const readSaved = key => {try{return JSON.parse(localStorage.getItem(key));}catch{return null;}};
const save = (key,value) => {try{localStorage.setItem(key,JSON.stringify(value));}catch{/* Playing remains available without browser storage. */}};
const forget = key => {try{localStorage.removeItem(key);}catch{/* Nothing to remove without browser storage. */}};
/** What the page being shown must undo when another page replaces it: its listeners on the window and document, and its timers. */
const cleanups = [];
const onLeave = run => {cleanups.push(run);};
const listen = (target,type,listener,options) => {target.addEventListener(type,listener,options);onLeave(()=>target.removeEventListener(type,listener,options));};
const every = (run,ms) => {const id=setInterval(run,ms);onLeave(()=>clearInterval(id));};
/** Sets the tab title and the description from the page's entry in `pageMeta`. */
function setMeta(game,page){const meta=pageMeta(game,page);document.title=meta.title;$('meta[name="description"]')?.setAttribute('content',meta.description);}
const icons = {
  undo:'<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 7h6a3 3 0 0 1 0 6H8M6.5 4.5 4 7l2.5 2.5"/></svg>',
  redo:'<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M12 7H6a3 3 0 0 0 0 6h2M9.5 4.5 12 7l-2.5 2.5"/></svg>',
};

// Theme follows the system until the visitor picks one; picking the system's own theme returns to following it.
const themeButton=$('#theme'),systemDark=matchMedia('(prefers-color-scheme: dark)');
const storedTheme=()=>{try{return localStorage.getItem('pencil-theme');}catch{return null;}};
function themeLabel(){const label=document.documentElement.dataset.theme==='dark'?'Switch to light mode':'Switch to dark mode';themeButton.setAttribute('aria-label',label);$('#theme-text').textContent=document.documentElement.dataset.theme==='dark'?'Light mode':'Dark mode';}
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
  /** Abandons the calls a page that is being left is still waiting for. Their promises never settle. */
  drop(){if(!this.pending.size)return;this.worker?.terminate();this.worker=null;for(const p of this.pending.values())clearTimeout(p.timer);this.pending.clear();}
}
const engine=new Engine();
function errorMessage(error){return /budget/i.test(error.message)?'This puzzle needs more work than the site allows. Try a smaller size, a lower difficulty or another puzzle.':error.message;}

/** How selecting a cell highlights others, read from the view: boards with blocks (Kakuro) use runs, digit boards with regions (Sudoku) use rows, columns and regions, boards whose editable cells have labels (Hitori) use rows and columns, other number boards (Fillomino) use the connected group of equal numbers. */
function highlightMode(view){
  if(view.cells.some(c=>c.block))return 'runs';
  if(view.marks.length>3&&view.cells.some(c=>c.region!==null&&c.region!==undefined))return 'regions';
  if(view.cells.some(c=>c.label&&c.editable))return 'lines';
  return view.marks.length>3?'group':'none';
}
/** Whether the board has walls between regions, as Aquarium has. Such a board has no selected cell: a ring would lie on the walls. */
const hasWalls=view=>{const region=i=>view.cells[i].region??null;return highlightMode(view)!=='regions'&&view.cells.some((c,i)=>(i%view.cols>0&&region(i)!==region(i-1))||(i>=view.cols&&region(i)!==region(i-view.cols)));};
/** The cells of the run through `index` in direction (dr, dc), up to the nearest blocks. */
function run(view,index,dr,dc){
  const cells=[index],r0=Math.floor(index/view.cols),c0=index%view.cols;
  for(const s of [-1,1])for(let r=r0+s*dr,c=c0+s*dc;r>=0&&r<view.rows&&c>=0&&c<view.cols&&!view.cells[r*view.cols+c].block;r+=s*dr,c+=s*dc)cells.push(r*view.cols+c);
  return cells;
}
/** The other cells in either run through `index`. */
function runCells(view,index){const cells=new Set([...run(view,index,0,1),...run(view,index,1,0)]);cells.delete(index);return cells;}
/** The block holding the clue for the run through `index` in direction (dr, dc), or -1. */
function clueBlock(view,index,dr,dc){let r=Math.floor(index/view.cols)-dr,c=index%view.cols-dc;while(r>=0&&c>=0&&!view.cells[r*view.cols+c].block){r-=dr;c-=dc;}return r>=0&&c>=0?r*view.cols+c:-1;}
/** Cells whose digit must differ from the digit at `index`. */
function peerCells(view,index,mode){
  if(mode==='runs')return runCells(view,index);
  const peers=new Set();if(mode!=='regions')return peers;
  const r0=Math.floor(index/view.cols),c0=index%view.cols,region=view.cells[index].region;
  view.cells.forEach((cell,j)=>{if(j!==index&&(Math.floor(j/view.cols)===r0||j%view.cols===c0||cell.region===region))peers.add(j);});
  return peers;
}
/** The cells joined to `index` by sides through cells with the same value. */
function groupCells(view,index){
  const value=view.cells[index].value,group=new Set([index]),todo=[index];
  while(todo.length){const j=todo.pop(),r=Math.floor(j/view.cols),c=j%view.cols;for(const [dr,dc] of [[0,1],[0,-1],[1,0],[-1,0]]){const rr=r+dr,cc=c+dc,k=rr*view.cols+cc;if(rr>=0&&rr<view.rows&&cc>=0&&cc<view.cols&&!group.has(k)&&view.cells[k].value===value){group.add(k);todo.push(k);}}}
  return group;
}
/** A mark's symbol as an element: dots and crosses are drawn shapes, other symbols are text. */
function swatch(mark){
  const shape=mark.symbol==='·'?' dot':mark.symbol==='×'?' cross':'';
  const s=element('span',shape||mark.shaded?'':mark.symbol,'swatch'+(mark.shaded?' shaded':'')+(mark.tone?' tone-'+mark.tone:'')+shape);s.setAttribute('aria-hidden','true');return s;
}
function board(view,{interactive=false,notes,selected=-1,selection,wrong}={}){
  const table=element('table',undefined,'board');table.setAttribute('aria-label',interactive?'Puzzle board':'Puzzle solution');
  const walled=hasWalls(view);
  const left=view.left.length>0, right=view.right.length>0, blank=view.marks[0]?.value, ends=linkEnds(view), mode=highlightMode(view);
  const first=view.cells.findIndex(cell=>cell.editable), focus=selected>=0?selected:first>=0||!ends.size?first:Math.min(...ends.keys());
  table.style.setProperty('--cols',String(view.cols+(left?1:0)+(right?1:0)));
  const chosen=selection??new Set(selected>=0?[selected]:[]);
  const from=chosen.size<=1&&selected>=0&&!view.cells[selected].block?view.cells[selected]:null;
  const sr=from?Math.floor(selected/view.cols):-1, sc=from?selected%view.cols:-1;
  let peers=new Set(),same=new Set(),across=-1,down=-1;
  if(from&&interactive){
    peers=peerCells(view,selected,mode);
    const value=from.label||(from.value!==blank?from.value:null);
    if(value!==null&&mode==='group')same=groupCells(view,selected);
    else if(value!==null&&mode!=='none')view.cells.forEach((cell,i)=>{
      if(cell.block||(mode==='lines'?cell.label:cell.value)!==value)return;
      if(mode==='regions'||i===selected||(mode==='runs'&&peers.has(i))||(mode==='lines'&&(Math.floor(i/view.cols)===sr||i%view.cols===sc)))same.add(i);
    });
    if(mode==='runs'){across=clueBlock(view,selected,0,1);down=clueBlock(view,selected,1,0);}
  }
  // Bridges at each island, so islands can show when they have their number.
  const bridges=new Map();
  if(interactive)(view.links||[]).forEach(l=>{const lines=view.link_marks?.find(m=>m.value===l.value)?.lines??0,p=l.place.link;if(lines)for(const j of [p.row*view.cols+p.col,p.row2*view.cols+p.col2])bridges.set(j,(bridges.get(j)||0)+lines);});
  const header=(text,on,className='')=>{const th=element('th',text,(className+(on?' hl':'')).trim()||undefined);return th;};
  if(view.top.length){const row=document.createElement('tr');if(left)row.append(element('th',''));for(let c=0;c<view.cols;c++){const th=header(view.top[c]||'',c===sc);th.scope='col';row.append(th);}if(right)row.append(element('th',''));table.append(row);}
  for(let r=0;r<view.rows;r++){
    const row=document.createElement('tr');if(left){const th=header(view.left[r]||'',r===sr);th.scope='row';row.append(th);}
    for(let c=0;c<view.cols;c++){
      const i=r*view.cols+c,cell=view.cells[i],choice=view.marks.find(m=>m.value===cell.value),noted=!!notes?.[i]&&cell.value===blank;
      const td=document.createElement('td');td.dataset.cellRow=String(r);td.dataset.cellCol=String(c);if(cell.region!==null&&cell.region!==undefined)td.dataset.region=String(cell.region);
      if(view.row_separators?.includes(r))td.classList.add('box-top');if(view.col_separators?.includes(c))td.classList.add('box-left');
      // Neighbouring cells of different regions, such as Aquarium tanks, get a wall between them, unless a separator is already there.
      const region=j=>view.cells[j].region??null;if(r>0&&region(i)!==region(i-view.cols)&&!td.classList.contains('box-top'))td.classList.add('wall-top');if(c>0&&region(i)!==region(i-1)&&!td.classList.contains('box-left'))td.classList.add('wall-left');
      if(r===0)td.classList.add('edge-top');if(r===view.rows-1)td.classList.add('edge-bottom');if(c===0)td.classList.add('edge-left');if(c===view.cols-1)td.classList.add('edge-right');
      if(cell.block){
        // A block: shaded, and split by a diagonal when it holds labels.
        const block=element('span','','cell block'+(chosen.has(i)?' selected':''));block.dataset.index=String(i);block.setAttribute('aria-label',`Row ${r+1}, column ${c+1}: ${cell.block.name}`);
        if(cell.block.upper||cell.block.lower){block.classList.add('split');block.append(element('span',cell.block.upper,'upper'+(i===across?' hl':'')),element('span',cell.block.lower,'lower'+(i===down?' hl':'')));}
        td.append(block);row.append(td);continue;
      }
      const label=cell.label||'',symbol=choice?.symbol??cell.value,dot=!!choice&&symbol==='·',cross=!!choice&&symbol==='×';
      let className='cell'+(choice?.shaded?' shaded':'')+(choice?.tone?' tone-'+choice.tone:'')+(dot?(label?' circled':' dot'):'')+(cross?' cross':'')+(label?' labelled':'')+(!cell.editable?' given':'')+(chosen.has(i)?' selected':'')+(same.has(i)?' same':'')+(peers.has(i)?' peer':'')+(ends.has(i)?' end':'')+(wrong?.has(i)?' wrong':'');
      if(cell.circled&&interactive&&/^\d+$/.test(cell.value)&&ends.size){const n=bridges.get(i)||0,want=Number(cell.value);className+=n===want?' complete':n>want?' over':'';}
      // Text sits in a glyph box, which CSS trims to the digits' height so they are centred in the cell and in rings.
      const text=cell.circled?'':label||(dot||cross?'':symbol),glyph=t=>element('span',t,'glyph');
      const button=element(interactive?'button':'span',undefined,className);button.dataset.index=String(i);if(text)button.append(glyph(text));
      if(cell.circled){const circle=element('span',undefined,'circle');circle.append(glyph(cell.value));button.append(circle);}
      if(noted){const set=new Set(notes[i].split(',')),grid=element('span',undefined,'notes');grid.style.setProperty('--note-cols',String(Math.ceil(Math.sqrt(view.marks.length-1))));for(const m of view.marks.slice(1))grid.append(element('span',set.has(m.value)?m.symbol:''));button.replaceChildren(grid);}
      const state=noted?'notes '+notes[i]:choice?.name??cell.value;
      button.setAttribute('aria-label',`Row ${r+1}, column ${c+1}: ${label?`${label}, ${state.toLowerCase()}`:state}`);
      if(interactive){button.type='button';button.tabIndex=i===focus&&!walled?0:-1;}
      td.append(button);row.append(td);
    }
    if(right){const clue=header(view.right[r]||'',r===sr,'clue');clue.dataset.row=String(r);row.append(clue);}table.append(row);
  }
  if(view.bottom.length){const row=document.createElement('tr');if(left)row.append(element('th',''));for(let c=0;c<view.cols;c++){const clue=header(view.bottom[c]||'',c===sc,'clue');clue.dataset.col=String(c);row.append(clue);}if(right)row.append(element('th',''));table.append(row);}
  if(table.querySelector('.wall-top,.wall-left'))table.classList.add('walled');
  return view.links?.length?linked(table,view,interactive,wrong):table;
}
// Links between cell centres (I4), drawn as lines over the table once it has a size.
const svgNS='http://www.w3.org/2000/svg';
const svg=(tag,attrs={})=>{const e=document.createElementNS(svgNS,tag);for(const [k,v] of Object.entries(attrs))e.setAttribute(k,String(v));return e;};
const linkKey=p=>`${p.row},${p.col},${p.row2},${p.col2}`;
/** Each cell at the end of a link, with the links leaving it: link index and the cell at the other end. */
function linkEnds(view){
  const ends=new Map(),add=(i,k,other)=>{if(!ends.has(i))ends.set(i,[]);ends.get(i).push({k,other});};
  (view.links||[]).forEach((link,k)=>{const p=link.place.link,a=p.row*view.cols+p.col,b=p.row2*view.cols+p.col2;add(a,k,b);add(b,k,a);});
  return ends;
}
// Players reach links with the mouse, or from the cells at their ends with the keyboard.
function linked(table,view,interactive,wrong){
  table.classList.add('linked');
  const wrap=element('div',undefined,'board-wrap'),layer=svg('svg',{class:'links','aria-hidden':'true'});
  const groups=view.links.map((link,k)=>{
    const p=link.place.link,mark=view.link_marks.find(m=>m.value===link.value),lines=mark?.lines??0;
    const g=svg('g',{class:'link'+(link.editable&&interactive?' editable':'')+(lines?'':' empty')+(wrong?.has(view.cells.length+k)?' wrong':'')});
    g.dataset.link=String(k);g.dataset.linkPlace=linkKey(p);
    layer.append(g);return {g,p,lines};
  });
  wrap.append(table,layer);
  const draw=()=>{
    if(!wrap.isConnected){observer.disconnect();return;}
    const box=wrap.getBoundingClientRect();layer.setAttribute('width',String(box.width));layer.setAttribute('height',String(box.height));
    const centre=(r,c)=>{const td=table.querySelector(`td[data-cell-row="${r}"][data-cell-col="${c}"]`).getBoundingClientRect();return {x:td.left+td.width/2-box.left,y:td.top+td.height/2-box.top,size:td.width,circled:view.cells[r*view.cols+c].circled};};
    for(const {g,p,lines} of groups){
      const a=centre(p.row,p.col),b=centre(p.row2,p.col2),length=Math.hypot(b.x-a.x,b.y-a.y),dx=(b.x-a.x)/length,dy=(b.y-a.y)/length;
      const cut=end=>end.circled?end.size*.4:0,x1=a.x+dx*cut(a),y1=a.y+dy*cut(a),x2=b.x-dx*cut(b),y2=b.y-dy*cut(b),gap=a.size*.1;
      const children=[svg('line',{class:'hit',x1,y1,x2,y2,'stroke-width':a.size*.36}),svg('line',{class:'guide',x1,y1,x2,y2})];
      for(let i=0;i<lines;i++){const o=(i-(lines-1)/2)*gap*2;children.push(svg('line',{class:'bridge',x1:x1-dy*o,y1:y1+dx*o,x2:x2-dy*o,y2:y2+dx*o}));}
      g.replaceChildren(...children);
    }
  };
  const observer=new ResizeObserver(draw);observer.observe(table);
  return wrap;
}
function directory(catalogue,current) {
  const nav=$('#directory');
  nav.replaceChildren(link('Overview','./',current.page==='home'),element('p','Games','label'));
  for(const game of catalogue){
    const id=game.metadata.id,open=current.game===game,wrap=element('div',undefined,'game'+(open?' open':''));
    wrap.append(link(game.metadata.name,`${id}/player/`,open));
    nav.append(wrap);
  }
  if(current.game){const sep=()=>element('span','/','sep'),here=element('span',current.page==='solver'?'Solver':'Player');here.setAttribute('aria-current','page');$('#trail').append(sep(),link(current.game.metadata.name,`${current.game.metadata.id}/player/`,false),sep(),here);}
  const toggle=$('#show-games'),isOpen=()=>document.body.classList.contains('directory-open');
  const setOpen=open=>{document.body.classList.toggle('directory-open',open);toggle.setAttribute('aria-expanded',String(open));};
  toggle.onclick=()=>setOpen(!isOpen());
  listen(document,'click',e=>{if(isOpen()&&!e.target.closest('#sidebar,#show-games'))setOpen(false);});
  listen(document,'keydown',e=>{if(e.key==='Escape'&&isOpen()){setOpen(false);toggle.focus();}});
}
function home(catalogue){
  setMeta();
  main.innerHTML=`<h1>pencil-puzzles</h1>
<p class="lede">Generate and play pencil puzzles from easy to expert, or solve one of your own.</p>
<div class="game-grid" id="games"></div>
<h2>About</h2>
<div class="prose"><p>pencil-puzzles is an open-source engine for generating, solving and rating pencil puzzles, available as a Python package and a command-line tool. This site runs the same engine in the browser. Every generated puzzle has exactly one solution, and its level, from easy to expert, is the level of the hardest technique needed to solve it.</p></div>`;
  // Each game is a tile: its solved example board, which the build stores in the catalogue, above its name.
  for(const game of catalogue){
    const m=game.metadata,tile=element('a',undefined,'game-tile'),picture=element('div',undefined,'game-picture');
    tile.href=`${m.id}/player/`;picture.setAttribute('aria-hidden','true');picture.inert=true;if(game.picture)picture.append(board(game.picture));
    tile.append(picture,element('span',m.name,'game-name'));$('#games').append(tile);
  }
}
function heading(game,page){const id=game.metadata.id;main.innerHTML=`<div class="title-row"><h1>${escape(game.metadata.name)}</h1><nav class="mode-links" aria-label="Game mode"><a id="tab-player" href="${id}/player/" ${page==='player'?'aria-current="page"':''}>Player</a><a id="tab-solver" href="${id}/solver/" ${page==='solver'?'aria-current="page"':''}>Solver</a></nav></div>`;setMeta(game,page);}
function status(message,kind=''){const box=$('#status');box.className='status'+(kind?' '+kind:'');box.textContent=message;}
function keyList(rows){return rows.flatMap(([keys,action])=>{const dt=element('dt');keys.split(' / ').forEach((k,i)=>{if(i)dt.append(' ');dt.append(element('kbd',k));});return [dt,element('dd',action)];});}
const capitalize=s=>s.charAt(0).toUpperCase()+s.slice(1);
function player(game){
  heading(game,'player');
  const id=game.metadata.id,sizes=game.sizes?.length?game.sizes:[game.params];
  const sizeOptions=sizes.map(s=>`<option value="${escape(s)}"${s===game.params?' data-standard':''}>${escape(s)}</option>`).join('')+(game.custom===false?'':'<option value="">Custom…</option>');
  main.insertAdjacentHTML('beforeend',`<section aria-label="Puzzle player"><fieldset id="controls" class="player-layout">
<div class="toolbar"><div class="field"><label for="size">Size</label><select id="size">${sizeOptions}</select><input id="params" value="${escape(game.params)}" spellcheck="false" autocomplete="off" aria-label="Custom size" hidden></div><div class="field"><label for="difficulty">Difficulty</label><select id="difficulty"><option value="easy">Easy</option><option value="medium">Medium</option><option value="hard">Hard</option><option value="expert">Expert</option></select></div><button id="new-game" type="button">New puzzle</button></div>
<div class="board-block"><div class="board-area"><div id="board"></div><button id="resume" type="button" hidden>Resume</button></div>
<div class="board-caption"><span class="stats"><span id="timer">0:00</span> · <span id="progress"></span><span id="rating"></span></span><span class="caption-tools"><button id="pause" class="link-button" type="button">Pause</button><button id="share" class="link-button" type="button">Copy link</button></span></div></div>
<div class="side">
<div id="legend" class="legend"></div>
<p id="combos" class="combos" aria-live="polite" hidden></p>
<p id="finish-note" class="finish-note"></p>
<div class="board-tools"><span class="group"><button id="undo" class="square" type="button" aria-label="Undo" title="Undo (Ctrl+Z)">${icons.undo}<span class="key-hint" aria-hidden="true">Z</span></button><button id="redo" class="square" type="button" aria-label="Redo" title="Redo (Ctrl+Y)">${icons.redo}<span class="key-hint" aria-hidden="true">Y</span></button></span><button id="clear" type="button" title="Clear all marks (undo restores them)">Clear</button><button id="check" class="primary push" type="button">Check</button></div>
<div class="assist"><label class="check"><input id="mistakes" type="checkbox">Show mistakes</label><button id="reveal" class="link-button" type="button" title="Undo returns to your marks">Show solution</button></div>
<p id="status" class="status" role="status" aria-live="polite">Loading…</p>
<div class="side-more"><details id="rules-box"><summary>Rules</summary><ul id="rules" class="rules"></ul></details><details class="keys-box"><summary>Keyboard shortcuts</summary><dl id="keys" class="keys"></dl></details><details><summary>Load puzzle</summary><label for="load-input">ID, text, or JSON</label><textarea id="load-input" maxlength="1000000" spellcheck="false" placeholder="${escape(game.example)}"></textarea><button id="load" type="button">Load</button></details></div>
</div></fieldset></section>`);
  const redrawSizes=customMenu($('#size')),redrawLevels=customMenu($('#difficulty'));
  // Levels the generator cannot make at the chosen size are grayed out, and so are the sizes it cannot make at the chosen level. A custom size is checked when generating.
  function availability(){
    const size=$('#params').value,difficulty=$('#difficulty'),levels=game.levels?.[size];
    if(levels?.length&&!levels.includes(difficulty.value))difficulty.value=levels.at(-1);
    for(const o of difficulty.options){o.disabled=!!levels&&!levels.includes(o.value);o.title=`Not available at ${size}`;}
    for(const o of $('#size').options){const ok=game.levels?.[o.value];o.disabled=!!ok&&!ok.includes(difficulty.value);o.title=`Not available at ${capitalize(difficulty.value)}`;}
    redrawSizes();redrawLevels();
  }
  const key='pencil-game:'+id,saveKey=puzzle=>`pencil-save:${id}:${puzzle}`,listKey='pencil-saves:'+id,stepsKey='pencil-history:'+id;
  let current=null,history=null,busy=false,entry=false,mode='none',selected=-1,selection=new Set(),notesMode=false,drag=null,linkDrag=null,solvedTime=null,checkTimer=0,checkSeq=0,ends=new Map();
  let tool=null,paused=false,revealed=false,rating=null,solution=null,showMistakes=readSaved('pencil-mistakes')===true;
  const clock={elapsed:0,since:null};const boardBox=$('#board');
  const blank=()=>current.view.marks[0].value;
  // Values are the cells in row-major order, then the links (I4).
  const cellCount=()=>current.view.cells.length,linkMarks=()=>current.view.link_marks||[];
  const linkEditable=k=>!!current.view.links?.[k]?.editable;
  function cycleLink(k,step){if(!linkEditable(k))return;const marks=linkMarks(),i=cellCount()+k,at=marks.findIndex(m=>m.value===history.values[i]);apply(i,marks[(at+step+marks.length)%marks.length].value);}
  const ahead=(i,j,[dr,dc])=>{const cols=current.view.cols;return Math.sign(Math.floor(j/cols)-Math.floor(i/cols))===dr&&Math.sign(j%cols-i%cols)===dc;};
  function linkToward(i,move,step){const out=ends.get(i)?.find(({other})=>ahead(i,other,move));if(out)cycleLink(out.k,step);}
  // On boards with links, only cells at the ends of links can be selected.
  const selectable=i=>!ends.size||ends.has(i);
  function select(i,add=false){if(hasWalls(current.view)){selected=-1;selection=new Set();return;}if(!add){selected=i;selection=new Set([i]);return;}if(selection.has(i)&&selection.size>1){selection.delete(i);if(selected===i)selected=[...selection].at(-1);}else{selection.add(i);selected=i;}}
  // Shading genres have one shaded mark and one dot mark; marking a cell with the mark it already has clears it.
  const shadeMark=()=>current.view.marks.find(m=>m.shaded)?.value,dotMark=()=>current.view.marks.find((m,i)=>i>0&&!m.shaded)?.value;
  // A click places the chosen mark (shaded until another is picked in the legend); a right-click places the other one.
  const primary=()=>tool??shadeMark(),secondary=()=>primary()===dotMark()?shadeMark():dotMark();
  const toggled=(i,value)=>history.values[i]===value?blank():value;const editable=i=>i>=0&&(i<cellCount()?current.view.cells[i].editable:linkEditable(i-cellCount()));
  const elapsed=()=>clock.elapsed+(clock.since===null?0:performance.now()-clock.since);
  function startClock(){if(current&&clock.since===null&&solvedTime===null&&!paused&&!document.hidden)clock.since=performance.now();}
  function pauseClock(){if(clock.since!==null){clock.elapsed+=performance.now()-clock.since;clock.since=null;}}
  const tick=()=>{const t=$('#timer');t.textContent=(solvedTime===null?'':revealed?'Shown · ':'Solved · ')+formatTime(solvedTime??elapsed());t.classList.toggle('solved',solvedTime!==null);};every(tick,1000);
  listen(document,'visibilitychange',()=>{if(document.hidden){pauseClock();persist();}else startClock();});listen(window,'pagehide',()=>persist());onLeave(()=>{clearTimeout(checkTimer);persist();});
  function sync(){
    const ready=!!current&&!busy&&!paused;
    $('#undo').disabled=!ready||!history?.past.length;$('#redo').disabled=!ready||!history?.future.length;
    for(const b of ['check','clear','reveal'])$('#'+b).disabled=!ready;
    $('#share').disabled=!current||busy;$('#mistakes').disabled=!current||busy;
    $('#pause').disabled=!current||busy||solvedTime!==null;$('#pause').textContent=paused?'Resume':'Pause';
    $('#load').disabled=busy;$('#load-input').disabled=busy;
  }
  // Each puzzle keeps its own save; the undo history is kept for the latest puzzle of each game.
  function persist(){
    if(!current)return;
    save(key,{id:current.id,params:$('#params').value,difficulty:$('#difficulty').value});
    save(saveKey(current.id),{marks:history.values,notes:history.notes,elapsed:elapsed(),solved:solvedTime,revealed,rating});
    save(stepsKey,{id:current.id,...history.steps(100)});
    const {list,dropped}=remember(readSaved(listKey),current.id,30);save(listKey,list);for(const d of dropped)forget(saveKey(d));
  }
  function showSize(value){const preset=[...$('#size').options].some(o=>o.value&&o.value===value);$('#params').value=value;$('#size').value=preset?value:'';$('#params').hidden=preset;availability();}
  /** Cells whose marks contradict the solution; blanks are never mistakes. */
  function mistakes(values){
    const wrong=new Set();if(!showMistakes||!solution?.unique||solution.id!==current.id)return wrong;
    const n=cellCount(),b=blank(),dot=dotMark(),lines=v=>linkMarks().find(m=>m.value===v)?.lines??0;
    values.forEach((v,i)=>{if(!editable(i))return;const s=solution.values[i];if(i<n){if(v!==b&&v!==(!entry&&s===b&&dot!==undefined?dot:s))wrong.add(i);}else if(lines(v)>lines(s))wrong.add(i);});
    return wrong;
  }
  function showCombos(values){
    const box=$('#combos'),view=current.view;
    if(mode!=='runs'||selection.size!==1||selected<0||view.cells[selected].block){box.hidden=true;return;}
    const max=Number(view.marks.at(-1).value),lines=[];
    for(const [name,dr,dc,part] of [['Across',0,1,'upper'],['Down',1,0,'lower']]){
      const b=clueBlock(view,selected,dr,dc),sum=b>=0?Number(view.cells[b].block[part]):0;if(!sum)continue;
      const cells=run(view,selected,dr,dc),placed=cells.map(i=>Number(values[i])).filter(Boolean),sets=combinations(sum,cells.length,max,placed);
      lines.push(`${name} ${sum} in ${cells.length}: ${sets.length?sets.map(s=>s.join('')).join(' '):'none'}`);
    }
    box.hidden=!lines.length;box.replaceChildren(...lines.flatMap((l,i)=>i?[element('br'),l]:[l]));
  }
  function draw(values=history.values,focus=boardBox.contains(document.activeElement)){
    const n=cellCount(),view={...current.view,cells:current.view.cells.map((cell,i)=>({...cell,value:values[i]})),links:(current.view.links||[]).map((link,k)=>({...link,value:values[n+k]}))};
    const table=board(view,{interactive:true,notes:history.notes,selected,selection,wrong:mistakes(values)});table.classList.add(entry?'entry':'paint');boardBox.replaceChildren(table);
    if(focus&&selected>=0)boardBox.querySelector(`[data-index="${selected}"]`)?.focus({preventScroll:true});
    const linkBlank=linkMarks()[0]?.value;
    $('#progress').textContent=view.links.length?`${view.links.filter(l=>l.editable&&l.value!==linkBlank).length} drawn`:`${values.filter((v,i)=>i<n&&view.cells[i].editable&&v!==current.view.cells[i].value).length} / ${view.cells.filter(c=>c.editable).length} cells`;
    // With regions (Sudoku), a digit placed once in every row is complete.
    if(mode==='regions'){const counts=new Map();for(let i=0;i<n;i++)counts.set(values[i],(counts.get(values[i])||0)+1);for(const b of $('#legend').querySelectorAll('[data-value]'))b.classList.toggle('done',(counts.get(b.dataset.value)||0)>=view.rows);}
    for(const b of $('#legend').querySelectorAll('[data-tool]'))b.setAttribute('aria-pressed',String(b.dataset.tool===primary()));
    showCombos(values);sync();
  }
  function changed(){draw();persist();status('');scheduleCheck();}
  function commit(values,notes=history.notes){if(!paused&&history.set(values,notes))changed();}
  function apply(i,value){if(!editable(i))return;const values=[...history.values];values[i]=value;commit(values);}
  // Number entry goes to every selected cell. A digit every selected cell already has erases it, and placing a digit removes it from the notes of cells that can't hold it.
  function enter(value,asNote){
    const targets=[...selection].filter(editable);if(!targets.length)return;
    const values=[...history.values],notes=[...history.notes],b=blank(),has=i=>notes[i].split(',').includes(value);
    if(value===b){if(targets.some(i=>values[i]!==b))for(const i of targets)values[i]=b;else for(const i of targets)notes[i]='';}
    else if(asNote){const open=targets.filter(i=>values[i]===b);if(!open.length)return;const all=open.every(has);for(const i of open)if(has(i)===all)notes[i]=toggleNote(notes[i],value,current.view.marks);}
    else{const erase=targets.every(i=>values[i]===value);for(const i of targets){values[i]=erase?b:value;if(!erase)for(const j of peerCells(current.view,i,mode))notes[j]=removeNote(notes[j],value);}}
    commit(values,notes);
  }
  function undo(){if(!paused&&history.undo())changed();}
  function redo(){if(!paused&&history.redo())changed();}
  function solved(){if(solvedTime===null){pauseClock();solvedTime=elapsed();tick();persist();}status(revealed?'Solution shown.':`Solved in ${formatTime(solvedTime)}.`,'success');sync();}
  function scheduleCheck(){clearTimeout(checkTimer);checkTimer=setTimeout(async()=>{const seq=++checkSeq,marks=history.values;if(entry&&marks.some((v,i)=>editable(i)&&v===blank()))return;try{const result=await engine.call({operation:'check',genre:id,puzzle:current.id,marks,finished:true});if(seq===checkSeq&&result.solved)solved();}catch{/* The Check button reports errors. */}},200);}
  function setNotes(on){notesMode=on;$('#notes')?.setAttribute('aria-pressed',String(on));}
  function setPaused(on){
    if(on===paused||!current||(on&&solvedTime!==null))return;
    paused=on;if(on)pauseClock();else startClock();
    boardBox.classList.toggle('paused',on);$('#resume').hidden=!on;tick();sync();
    if(on)$('#resume').focus();else boardBox.querySelector(`[data-index="${selected}"]`)?.focus({preventScroll:true});
  }
  async function ensureSolution(){
    if(solution?.id===current.id)return solution;
    const puzzle=current.id,result=await engine.call({operation:'solve',genre:id,puzzle}),v=result.views[0];
    const found={id:puzzle,unique:result.status==='unique',values:v?[...v.cells,...(v.links||[])].map(c=>c.value):null};
    if(current?.id===puzzle)solution=found;return found;
  }
  async function loadSolution(){try{const s=await ensureSolution();if(!s.unique)status('This puzzle does not have exactly one solution, so mistakes cannot be shown.','error');else if(current?.id===s.id)draw();}catch(e){status(errorMessage(e),'error');}}
  boardBox.addEventListener('contextmenu',e=>{if(e.target.closest('.cell,.link'))e.preventDefault();});
  boardBox.addEventListener('pointerdown',e=>{
    if(busy||!current||paused||(e.button!==0&&e.button!==2))return;
    const link=e.target.closest('.link.editable');
    if(link){e.preventDefault();cycleLink(Number(link.dataset.link),e.button===2?-1:1);return;}
    const cell=e.target.closest('.cell');if(!cell)return;const i=Number(cell.dataset.index);if(!selectable(i))return;
    e.preventDefault();
    // Islands: dragging from one toward another changes the link between them.
    if(ends.size){select(i);linkDrag={from:i,x:e.clientX,y:e.clientY,step:e.button===2?-1:1,size:cell.getBoundingClientRect().width};draw(undefined,true);return;}
    if(entry){if(e.shiftKey||e.ctrlKey||e.metaKey)select(i,true);else{select(i);drag={select:true};}draw(undefined,true);return;}
    select(i);
    const target=e.button===2?secondary():primary();if(!editable(i)||target===undefined){draw(undefined,true);return;}
    drag={value:target===blank()?target:toggled(i,target),values:[...history.values]};drag.values[i]=drag.value;draw(drag.values,true);
  });
  listen(window,'pointermove',e=>{
    if(!drag)return;const cell=document.elementFromPoint(e.clientX,e.clientY)?.closest?.('#board .cell');if(!cell)return;const i=Number(cell.dataset.index);
    if(drag.select){if(!selection.has(i)&&!current.view.cells[i].block){selection.add(i);selected=i;draw();}return;}
    if(editable(i)&&drag.values[i]!==drag.value){drag.values[i]=drag.value;draw(drag.values);}
  });
  const endDrag=e=>{
    if(linkDrag){const d=linkDrag;linkDrag=null;const dx=e.clientX-d.x,dy=e.clientY-d.y;if(e.type==='pointerup'&&Math.hypot(dx,dy)>d.size*.5)linkToward(d.from,Math.abs(dx)>Math.abs(dy)?[0,Math.sign(dx)]:[Math.sign(dy),0],d.step);}
    if(!drag)return;const done=drag;drag=null;if(!done.select)commit(done.values);
  };
  listen(window,'pointerup',endDrag);listen(window,'pointercancel',endDrag);
  listen(document,'keydown',e=>{
    if(!current||busy||e.target.closest?.('input,textarea,select'))return;
    const mod=e.ctrlKey||e.metaKey,k=e.key.toLowerCase(),view=current.view,inBoard=boardBox.contains(e.target);
    if(mod&&(k==='z'||k==='y')){e.preventDefault();k==='y'||e.shiftKey?redo():undo();return;}
    if(mod||e.altKey||paused)return;
    if(entry&&k==='n'){setNotes(!notesMode);return;}
    if(!inBoard&&!(selected>=0&&(e.target===document.body||e.target.closest?.('#legend'))))return;
    if(inBoard&&e.target.dataset.index&&Number(e.target.dataset.index)!==selected)select(Number(e.target.dataset.index));
    if(e.key==='Escape'&&selection.size>1){select(selected);draw(undefined,true);return;}
    const moves={ArrowLeft:[0,-1],ArrowRight:[0,1],ArrowUp:[-1,0],ArrowDown:[1,0],a:[0,-1],d:[0,1],w:[-1,0],s:[1,0]};
    const move=moves[e.key.length===1?k:e.key];
    if(move&&ends.size){
      // Boards with links: Shift changes the link leaving the selected cell that way; otherwise move to the next cell at a link's end.
      e.preventDefault();if(selected<0){select(Math.min(...ends.keys()));draw(undefined,true);return;}
      if(e.shiftKey){linkToward(selected,move,1);return;}
      const r0=Math.floor(selected/view.cols),c0=selected%view.cols;
      for(let r=r0+move[0],c=c0+move[1];r>=0&&r<view.rows&&c>=0&&c<view.cols;r+=move[0],c+=move[1])if(ends.has(r*view.cols+c)){select(r*view.cols+c);break;}
      draw(undefined,true);return;
    }
    if(move){
      e.preventDefault();const r=Math.floor(selected/view.cols)+move[0],c=selected%view.cols+move[1];
      // Shift extends the selection on number boards.
      if(r>=0&&r<view.rows&&c>=0&&c<view.cols){const i=r*view.cols+c;if(entry&&e.shiftKey){selection.add(i);selected=i;}else select(i);}
      draw(undefined,true);return;
    }
    const digit=/^(?:Digit|Numpad)(\d)$/.exec(e.code)?.[1];
    if(digit!==undefined||e.key==='Backspace'||e.key==='Delete'){const mark=view.marks[digit===undefined?0:Number(digit)];if(!mark)return;e.preventDefault();entry?enter(mark.value,notesMode||e.shiftKey):apply(selected,mark.value);return;}
    const keyMark=entry?undefined:{q:shadeMark(),e:dotMark(),r:blank()}[k];
    if(keyMark!==undefined&&selected>=0){e.preventDefault();apply(selected,k==='r'?keyMark:toggled(selected,keyMark));}
  });
  async function action(message,task){if(busy)return;busy=true;$('#controls').disabled=true;sync();status(message);try{await task();}catch(e){status(errorMessage(e),'error');}finally{busy=false;$('#controls').disabled=false;sync();}}
  function showPuzzle(value,push){const url=location.pathname+`?puzzle=${encodeURIComponent(value.id)}`;if(push&&new URLSearchParams(location.search).get('puzzle')!==value.id)window.history.pushState(null,'',url);else window.history.replaceState(null,'',url);}
  function accept(value,{push=false}={}){
    const active=document.activeElement,focus=!active||active===document.body||active.matches('#new-game,#load,#board *');
    pauseClock();if(current)persist();
    checkSeq++;current=value;entry=value.view.marks.length>3;mode=highlightMode(value.view);drag=null;linkDrag=null;ends=linkEnds(value.view);solution=null;tool=null;paused=false;
    boardBox.classList.remove('paused');$('#resume').hidden=true;
    const items=[...value.view.cells,...(value.view.links||[])],cells=value.view.cells.length;
    const initial=items.map(c=>c.value);const allowed=new Set(value.view.marks.map(m=>m.value)),allowedLinks=new Set((value.view.link_marks||[]).map(m=>m.value));
    const validMarks=m=>Array.isArray(m)&&m.length===initial.length&&m.every((v,i)=>items[i].editable?(i<cells?allowed:allowedLinks).has(v):v===initial[i]);
    const validNotes=n=>Array.isArray(n)&&n.length===initial.length&&n.every(x=>typeof x==='string');
    // Saves from before per-puzzle saves held the marks next to the puzzle ID.
    const legacy=readSaved(key),saved=readSaved(saveKey(value.id))??(legacy?.id===value.id&&legacy.marks?legacy:null);
    const valid=validMarks(saved?.marks);
    history=new History(valid?saved.marks:initial,valid&&validNotes(saved.notes)?saved.notes:initial.map(()=>''));
    const steps=readSaved(stepsKey);if(valid&&steps?.id===value.id)history.restore(steps,([m,n])=>validMarks(m)&&validNotes(n));
    clock.elapsed=valid&&Number.isFinite(saved.elapsed)?saved.elapsed:0;clock.since=null;solvedTime=valid&&Number.isFinite(saved.solved)?saved.solved:null;revealed=valid&&saved.revealed===true;
    rating=value.rating?.level??(valid&&typeof saved.rating==='string'?saved.rating:null);
    if(value.params)showSize(value.params);
    $('#rules').replaceChildren(...value.view.instructions.split('\n').filter(Boolean).map(t=>element('li',t)));
    $('#finish-note').textContent=value.view.finish_note||'';
    const marks=value.view.marks,links=value.view.link_marks||[],linking=(value.view.links||[]).some(l=>l.editable);
    const shadeName=marks.find(m=>m.shaded)?.name.toLowerCase(),dotName=marks.find((m,i)=>i>0&&!m.shaded)?.name.toLowerCase();
    $('#keys').replaceChildren(...keyList(linking
      ?[['Click','next mark'],['Right-click','previous mark'],['Drag','from an island toward another'],['Arrows / WASD','move'],['Shift+Arrows / WASD','next mark that way'],['Ctrl+Z / Ctrl+Y','undo / redo']]
      :entry
      ?[[`${marks[1].symbol}–${marks.at(-1).symbol}`,'enter or erase'],['Shift+digit','note'],['N','notes mode'],['Del','erase'],['Arrows / WASD','move'],['Drag / Shift+click','select several'],['Esc','select one'],['Ctrl+Z / Ctrl+Y','undo / redo']]
      :hasWalls(value.view)
      // Boards with walls have no selected cell, so they are played with the pointer only.
      ?[['Click','chosen mark'],['Right-click','other mark'],['Drag','mark several'],['Ctrl+Z / Ctrl+Y','undo / redo']]
      :[['Click','chosen mark'],['Right-click','other mark'],['Q',shadeName],['E',dotName],['R / Del','clear'],['Drag','mark several'],['Arrows / WASD','move'],['Ctrl+Z / Ctrl+Y','undo / redo']]));
    const legendButton=(text,label,click)=>{const button=element('button',text);button.type='button';button.setAttribute('aria-label',label);button.onclick=click;button.onmousedown=e=>e.preventDefault();return button;};
    $('#legend').className='legend'+(entry?' digits':linking?'':' tools');
    if(linking)$('#legend').replaceChildren(...links.map(m=>{const label=element('span');const line=element('span','','swatch line lines-'+m.lines);line.setAttribute('aria-hidden','true');label.append(line,document.createTextNode(m.name));return label;}));
    else if(entry){
      const notesButton=legendButton('Notes','Notes mode',()=>setNotes(!notesMode));notesButton.id='notes';
      const pad=element('div',undefined,'keypad');pad.style.setProperty('--keypad-cols',String(Math.ceil(Math.sqrt(marks.length-1))));pad.append(...marks.slice(1).map(m=>{const b=legendButton(m.symbol,`Enter ${m.name}`,()=>enter(m.value,notesMode));b.dataset.value=m.value;return b;}));
      const tools=element('div',undefined,'keypad-tools');const eraseButton=legendButton('Erase','Erase cell',()=>enter(blank(),false));notesButton.append(keyHint('N'));eraseButton.append(keyHint('Del'));tools.append(notesButton,eraseButton);
      $('#legend').replaceChildren(pad,tools);
    }
    else{
      const group=element('div',undefined,'mark-tools');group.setAttribute('role','group');group.setAttribute('aria-label','Mark placed by a click');
      group.append(...marks.map(m=>{const b=legendButton('',m.name,()=>{tool=m.value;draw();});b.dataset.tool=m.value;b.append(swatch(m),document.createTextNode(m.name));const key=hasWalls(value.view)?'':m.shaded?'Q':m===marks[0]?'R':m===marks.find((x,i)=>i>0&&!x.shaded)?'E':'';if(key){const hint=element('kbd',key);hint.setAttribute('aria-hidden','true');b.append(hint);}return b;}));
      $('#legend').replaceChildren(group);
    }
    setNotes(false);
    const first=value.view.cells.findIndex(c=>c.editable);select(ends.size?Math.min(...ends.keys()):first>=0?first:0);
    $('#tab-solver').href=`${id}/solver/?puzzle=${encodeURIComponent(value.id)}`;showPuzzle(value,push);
    $('#rating').textContent=rating?` · ${capitalize(rating)}`:'';
    draw(undefined,focus);startClock();tick();persist();if(solvedTime===null)status('');else solved();
    if(!rating){const puzzle=value.id;engine.call({operation:'rate',genre:id,puzzle}).then(r=>{if(current?.id===puzzle&&r?.level){rating=r.level;$('#rating').textContent=` · ${capitalize(rating)}`;persist();}}).catch(()=>{/* Puzzles without one solution have no rating. */});}
    if(showMistakes)loadSolution();
  }
  const load=(puzzle,push)=>action('Loading…',async()=>accept(await engine.call({operation:'load',genre:id,puzzle}),{push}));
  async function newGame(){await action('Loading…',async()=>{const bytes=new Uint32Array(2);crypto.getRandomValues(bytes);const seed=((BigInt(bytes[0])<<32n)|BigInt(bytes[1])).toString();accept(await engine.call({operation:'generate',genre:id,params:$('#params').value,difficulty:$('#difficulty').value,seed}),{push:true});});}
  // A game with moves in it is only replaced on a second press within three seconds.
  let armed=0;const newButton=$('#new-game'),disarm=()=>{clearTimeout(armed);armed=0;newButton.textContent='New puzzle';newButton.classList.remove('confirm');};
  newButton.onclick=()=>{const started=solvedTime===null&&history&&(history.past.length||history.future.length);if(started&&!armed){newButton.textContent='Press again';newButton.classList.add('confirm');armed=setTimeout(disarm,3000);return;}disarm();newGame();};
  newButton.onblur=disarm;onLeave(()=>clearTimeout(armed));
  $('#size').onchange=()=>{const size=$('#size').value;$('#params').hidden=!!size;if(size)$('#params').value=size;else{$('#params').focus();$('#params').select();}availability();};
  $('#difficulty').onchange=availability;
  $('#params').addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();newGame();}});
  $('#params').addEventListener('input',availability);
  $('#undo').onclick=undo;$('#redo').onclick=redo;
  $('#pause').onclick=()=>setPaused(!paused);$('#resume').onclick=()=>setPaused(false);
  $('#clear').onclick=()=>{const items=[...current.view.cells,...(current.view.links||[])];commit(items.map(c=>c.value),items.map(()=>''));status('Cleared. Undo restores it.');};
  $('#check').onclick=()=>action('Checking…',async()=>{const result=await engine.call({operation:'check',genre:id,puzzle:current.id,marks:history.values,finished:true});$('#board').querySelectorAll('.invalid').forEach(c=>c.classList.remove('invalid'));for(const v of result.violations){const p=v.place;let selector=p==='board'?'td':'';if(p&&typeof p==='object'){if('row'in p)selector=`[data-row="${p.row}"],[data-cell-row="${p.row}"]`;else if('col'in p)selector=`[data-col="${p.col}"],[data-cell-col="${p.col}"]`;else if('region'in p)selector=`[data-region="${p.region}"]`;else if('cell'in p)selector=`[data-index="${p.cell.row*current.view.cols+p.cell.col}"]`;else if('link'in p)selector=`[data-link-place="${linkKey(p.link)}"]`;}if(selector)$('#board').querySelectorAll(selector).forEach(c=>c.classList.add('invalid'));}if(result.solved)solved();else status('Not solved. Check the marked cells or clues.','error');});
  $('#mistakes').checked=showMistakes;
  $('#mistakes').onchange=()=>{showMistakes=$('#mistakes').checked;save('pencil-mistakes',showMistakes);if(showMistakes)loadSolution();else draw();};
  $('#reveal').onclick=()=>action('Solving…',async()=>{
    const s=await ensureSolution();if(!s.values){status('This puzzle has no solution.','error');return;}
    const b=blank(),dot=dotMark(),n=cellCount();revealed=true;
    commit(history.values.map((v,i)=>!editable(i)?v:i<n&&!entry&&s.values[i]===b&&dot!==undefined?dot:s.values[i]),history.notes.map(()=>''));
    status(s.unique?'Solution shown. Undo returns to your marks.':'One of several solutions shown. Undo returns to your marks.');
  });
  $('#load').onclick=()=>load($('#load-input').value,true);
  listen(window,'popstate',()=>{const puzzle=new URLSearchParams(location.search).get('puzzle');if(puzzle&&puzzle!==current?.id)load(puzzle,false);});
  let shareTimer=0;
  $('#share').onclick=async()=>{const button=$('#share');try{await navigator.clipboard.writeText(location.href);button.textContent='Copied';clearTimeout(shareTimer);shareTimer=setTimeout(()=>{button.textContent='Copy link';},1500);}catch{status('Copy the URL from the address bar.');}};
  const rulesBox=$('#rules-box'),rulesKey='pencil-rules:'+id,rulesOpen=readSaved(rulesKey);rulesBox.open=!!rulesOpen;rulesBox.ontoggle=()=>save(rulesKey,rulesBox.open);
  const saved=readSaved(key);showSize(saved?.params||game.params);if(saved?.difficulty&&[...$('#difficulty').options].some(o=>o.value===saved.difficulty))$('#difficulty').value=saved.difficulty;availability();
  const supplied=new URLSearchParams(location.search).get('puzzle')||saved?.id;
  if(supplied)load(supplied,false);else newGame();
  sync();
}
function solver(game){
  heading(game,'solver');
  const id=game.metadata.id,sizes=game.sizes?.length?game.sizes:[game.params];
  const sizeOptions=sizes.map(s=>`<option value="${escape(s)}"${s===game.params?' data-standard':''}>${escape(s)}</option>`).join('')+(game.custom===false?'':'<option value="">Custom…</option>');
  main.insertAdjacentHTML('beforeend',`<section aria-label="Puzzle solver"><fieldset id="solver-controls" class="solver-layout">
<div class="toolbar"><div class="field"><label for="editor-size">Size</label><select id="editor-size">${sizeOptions}</select><input id="editor-shape" value="${escape(game.params)}" spellcheck="false" autocomplete="off" aria-label="Custom size" hidden></div><button id="example" type="button">Example</button></div>
<div class="editor-block"><div id="editor-grid" class="board-area"></div>
<div class="actions"><button id="clear-clues" type="button">Clear</button><button id="solve" class="primary" type="button">Solve</button></div>
<p id="status" class="status" role="status" aria-live="polite"></p>
<div class="side-more solver-more"><details><summary>Import ID or text</summary><label for="puzzle-input">ID, text, or JSON</label><textarea id="puzzle-input" maxlength="1000000" spellcheck="false" placeholder="${escape(game.example)}"></textarea><button id="import" type="button">Import</button></details></div></div>
<div class="result-head result-part"><h2 id="result-title"></h2><a id="play-link" href="${id}/player/">Play this puzzle</a></div>
<div id="solutions" class="result-boards result-part" aria-label="Solver result"></div>
<p id="result-id" class="puzzle-id result-part"></p>
</fieldset></section>`);
  let editor=null;let busy=false;let fields=[];
  const showResult=on=>{$('#solver-controls').classList.toggle('has-result',on);if(!on)$('#tab-player').href=`${id}/player/`;};
  function invalidate(){showResult(false);status('');}
  function label(place){if('row'in place)return `Row ${place.row+1} clue`;if('col'in place)return `Column ${place.col+1} clue`;return `Row ${place.cell.row+1}, column ${place.cell.col+1}`;}
  // Arrow keys move to the nearest clue input that way; in text inputs, Left and Right first move through the text.
  function moveBetweenFields(e){
    const dir={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,-1],ArrowDown:[0,1]}[e.key],input=e.target.closest?.('.clue-input');
    if(!dir||!input||e.altKey||e.ctrlKey||e.metaKey||e.shiftKey)return;
    if(input.type==='text'&&dir[0]!==0){const at=dir[0]<0?0:input.value.length;if(input.selectionStart!==at||input.selectionEnd!==at)return;}
    const box=input.getBoundingClientRect(),x=box.left+box.width/2,y=box.top+box.height/2;let best=null,score=Infinity;
    for(const other of fields){if(other===input)continue;const r=other.getBoundingClientRect(),dx=r.left+r.width/2-x,dy=r.top+r.height/2-y,along=dx*dir[0]+dy*dir[1],across=Math.abs(dx*dir[1]+dy*dir[0]);if(along<=1||across>along)continue;const s=along+across*3;if(s<score){score=s;best=other;}}
    e.preventDefault();if(best){best.focus();try{best.select();}catch{/* Not every input type supports select(). */}}
  }
  function drawEditor(value){
    editor=value;$('#editor-shape').value=value.shape;syncSize();
    const table=board(value.view);table.setAttribute('aria-label','Puzzle clues');table.addEventListener('keydown',moveBetweenFields);
    fields=value.fields.map((field,i)=>{
      const input=document.createElement('input');if(field.pattern){input.type='text';input.pattern=field.pattern;input.maxLength=5;input.spellcheck=false;input.autocomplete='off';}else{input.type='number';input.inputMode='numeric';input.min=String(field.min);input.max=String(field.max);input.step='1';}input.required=!field.optional;input.value=field.value;input.className='clue-input';input.setAttribute('aria-label',label(field.place));input.dataset.field=String(i);input.oninput=invalidate;
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
  // The size menu opens an empty grid of the chosen size. Custom shows the text box, which Enter applies.
  customMenu($('#editor-size'));
  const syncSize=()=>{const shape=$('#editor-shape').value,preset=[...$('#editor-size').options].some(o=>o.value&&o.value===shape);$('#editor-size').value=preset?shape:'';$('#editor-shape').hidden=preset;};
  $('#editor-size').onchange=()=>{const size=$('#editor-size').value;$('#editor-shape').hidden=!!size;if(size){$('#editor-shape').value=size;open(null);}else{$('#editor-shape').focus();$('#editor-shape').select();}};
  $('#example').onclick=()=>open(game.example);
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

/** Replaces a select with a button and a list drawn by the page. The select stays in the document, hidden, and still holds the value and fires `change`. Disabled options are grayed out, show their title on hover and cannot be chosen. Returns a function that redraws the list after options change. */
function customMenu(select){
  const wrap=element('div',undefined,'menu'),button=element('button',undefined,'menu-button'),list=element('div',undefined,'menu-list');
  const name=document.querySelector(`label[for="${select.id}"]`)?.textContent||'',native=Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value');
  button.type='button';button.setAttribute('aria-haspopup','listbox');button.setAttribute('aria-expanded','false');
  list.setAttribute('role','listbox');list.setAttribute('aria-label',name);list.hidden=true;
  for(const option of select.options){
    const item=element('div',option.textContent,'menu-item');item.setAttribute('role','option');item.dataset.value=option.value;item.tabIndex=-1;
    if(option.hasAttribute('data-standard'))item.append(element('span','standard','menu-tag'));
    list.append(item);
  }
  const items=[...list.children];
  const sync=()=>{const value=native.get.call(select),text=select.selectedOptions[0]?.textContent||'';button.textContent=text;button.setAttribute('aria-label',name?`${name}: ${text}`:text);items.forEach((item,i)=>{const option=select.options[i];item.setAttribute('aria-selected',String(item.dataset.value===value));item.setAttribute('aria-disabled',String(option.disabled));item.title=option.disabled?option.title:'';});};
  // The page sets the select's value in code, which fires no event, so the setter updates the button.
  Object.defineProperty(select,'value',{configurable:true,get(){return native.get.call(select);},set(value){native.set.call(select,value);sync();}});
  const open=on=>{list.hidden=!on;button.setAttribute('aria-expanded',String(on));if(on)(list.querySelector('[aria-selected="true"]')||items[0]).focus();};
  const choose=item=>{if(item.getAttribute('aria-disabled')==='true')return;select.value=item.dataset.value;open(false);button.focus();select.dispatchEvent(new Event('change',{bubbles:true}));};
  button.onclick=()=>open(list.hidden);
  list.onclick=e=>{const item=e.target.closest('.menu-item');if(item)choose(item);};
  wrap.addEventListener('keydown',e=>{
    e.stopPropagation();
    const at=items.indexOf(document.activeElement);
    if(list.hidden){if(['ArrowDown','ArrowUp'].includes(e.key)){e.preventDefault();open(true);}return;}
    if(e.key==='Escape'){e.preventDefault();open(false);button.focus();}
    else if(e.key==='Enter'||e.key===' '){e.preventDefault();if(at>=0)choose(items[at]);}
    else if(e.key==='ArrowDown'||e.key==='ArrowUp'){e.preventDefault();items[Math.max(0,Math.min(items.length-1,at+(e.key==='ArrowDown'?1:-1)))].focus();}
    else if(e.key==='Home'||e.key==='End'){e.preventDefault();items.at(e.key==='Home'?0:-1).focus();}
  });
  wrap.addEventListener('focusout',e=>{if(!wrap.contains(e.relatedTarget))open(false);});
  select.hidden=true;wrap.append(button,list);select.after(wrap);sync();
  return sync;
}
/** A button's shortcut key, shown dim in its bottom right corner. */
function keyHint(text){const hint=element('span',text,'key-hint');hint.setAttribute('aria-hidden','true');return hint;}

try{
  const response=await fetch('catalogue.json');if(!response.ok)throw new Error('Could not load the list of games.');const catalogue=await response.json();
  const base=new URL(document.baseURI).pathname;let shown=null;
  /** Shows the page for the current address in place of the one on screen. */
  const show=()=>{
    for(const run of cleanups.splice(0))run();
    engine.drop();
    [...$('#trail').children].slice(1).forEach(node=>node.remove());
    document.body.classList.remove('directory-open');$('#show-games').setAttribute('aria-expanded','false');
    shown=location.pathname;
    const current=route(location.pathname.slice(base.length-1),catalogue);directory(catalogue,current);if(current.page==='home')home(catalogue);else if(current.page==='player')player(current.game);else if(current.page==='solver')solver(current.game);else{document.title='Page not found · pencil-puzzles';main.innerHTML='<h1>Page not found</h1><p>That game or page does not exist.</p><p><a href="./">Back to all games</a></p>';}
  };
  // Links between pages of the app change the address and show the page without loading a new document.
  document.addEventListener('click',e=>{
    if(e.defaultPrevented||e.button||e.metaKey||e.ctrlKey||e.shiftKey||e.altKey)return;
    const a=e.target.closest?.('a[href]');if(!a||a.target||a.hasAttribute('download'))return;
    const url=new URL(a.href),path=url.pathname.slice(base.length);
    if(url.origin!==location.origin||!url.pathname.startsWith(base)||path.startsWith('docs/')||/\.[a-z0-9]+$/i.test(path)||(url.pathname===location.pathname&&url.hash))return;
    e.preventDefault();if(url.href!==location.href)history.pushState(null,'',url);show();scrollTo(0,0);
  });
  addEventListener('popstate',()=>{if(location.pathname!==shown)show();});
  show();
}catch(e){main.replaceChildren(element('h1','Could not load the list of games'),element('p',errorMessage(e)));}
