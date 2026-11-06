import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import init,{request} from '../dist/wasm/pencil_puzzles_web.js';
await init({module_or_path:await readFile(new URL('../dist/wasm/pencil_puzzles_web_bg.wasm',import.meta.url))});
const call=data=>JSON.parse(request(JSON.stringify(data)));
test('compiled browser engine solves and checks every registered game',()=>{
  for(const game of call({operation:'list'}).value){
    const genre=game.metadata.id;
    const generated=call({operation:'generate',genre,params:game.params,difficulty:'easy',seed:'2'});
    assert.equal(generated.ok,true);
    const puzzle=generated.value.id;
    const solved=call({operation:'solve',genre,puzzle});
    assert.equal(solved.value.status,'unique');
    const view=solved.value.views[0],marks=[...view.cells,...view.links].map(c=>c.value);
    assert.equal(call({operation:'check',genre,puzzle,marks,finished:true}).value.solved,true);
    assert.equal(call({operation:'check',genre,puzzle,marks:[],finished:true}).ok,false);
  }
});
test('compiled engine reports invalid, impossible and ambiguous inputs',()=>{
  const solve=puzzle=>call({operation:'solve',genre:'kakurasu',puzzle});
  assert.equal(solve('bad').ok,false);
  assert.equal(solve('kakurasu:1x1:1/0').value.status,'none');
  const multiple=solve('kakurasu:4x4:5.5.5.5/5.5.5.5');
  assert.equal(multiple.value.status,'multiple');
  assert.notEqual(multiple.value.answers[0],multiple.value.answers[1]);
});

test('visual clue entry produces the same puzzle and solution as ID input',()=>{
  const editor=call({operation:'editor',genre:'kakurasu',shape:'2x2',puzzle:null}).value;
  assert.equal(editor.fields.length,4);
  assert.equal(editor.fields[0].max,3);
  const puzzle=call({operation:'compose',genre:'kakurasu',shape:editor.shape,values:['1','2','1','2']});
  assert.equal(puzzle.value.id,'kakurasu:2x2:1.2/1.2');
  const solved=call({operation:'solve',genre:'kakurasu',puzzle:puzzle.value.id});
  assert.equal(solved.value.status,'unique');
  const imported=call({operation:'editor',genre:'kakurasu',shape:'6x6',puzzle:puzzle.value.id}).value;
  assert.equal(imported.shape,'2x2');
  assert.deepEqual(imported.fields.map(f=>f.value),['1','2','1','2']);
  assert.equal(call({operation:'compose',genre:'kakurasu',shape:'2x2',values:['4','2','1','2']}).ok,false);
});

test('Sudoku givens can be entered visually and empty answers do not pass',()=>{
  const editor=call({operation:'editor',genre:'sudoku',shape:'4x4',puzzle:null}).value;
  assert.equal(editor.fields.length,16);
  const values=['1','','','4','','4','','','','','4','','4','','','1'];
  const puzzle=call({operation:'compose',genre:'sudoku',shape:'4x4',values});
  assert.equal(puzzle.ok,true);
  assert.equal(puzzle.value.params,'4x4');
  assert.equal(puzzle.value.view.col_separators[0],2);
  const checked=call({operation:'check',genre:'sudoku',puzzle:puzzle.value.id,marks:puzzle.value.view.cells.map(c=>c.value),finished:true});
  assert.equal(checked.value.solved,false);
  assert.ok(checked.value.violations.length>0);
  const solved=call({operation:'solve',genre:'sudoku',puzzle:puzzle.value.id});
  assert.ok(['unique','multiple'].includes(solved.value.status));
  assert.ok(solved.value.views[0].cells.every(c=>c.value!=='.'));
  assert.equal(puzzle.value.view.cells[0].editable,false);
  assert.equal(puzzle.value.view.marks[1].shaded,false);
});

 test('Nurikabe visual clues, fixed numbers and sea shading share the engine',()=>{
   const genre='nurikabe';
   const editor=call({operation:'editor',genre,shape:'3x3',puzzle:null}).value;
   assert.equal(editor.fields.length,9);assert.equal(editor.fields[0].max,9);
   const puzzle=call({operation:'compose',genre,shape:'3x3',values:['1','','1','','','','1','','1']}).value;
   const solved=call({operation:'solve',genre,puzzle:puzzle.id}).value;
   assert.equal(solved.status,'unique');assert.equal(puzzle.view.cells[0].editable,false);
   assert.equal(puzzle.view.cells[0].value,'1');assert.equal(puzzle.view.marks[1].shaded,true);
   const marks=solved.views[0].cells.map(c=>c.value);
   assert.equal(call({operation:'check',genre,puzzle:puzzle.id,marks,finished:true}).value.solved,true);
   marks[0]='#';assert.equal(call({operation:'check',genre,puzzle:puzzle.id,marks,finished:true}).ok,false);
   assert.equal(call({operation:'check',genre,puzzle:puzzle.id,marks:puzzle.view.cells.map(c=>c.value),finished:true}).value.solved,false);
 });
