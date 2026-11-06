// Generation times in the compiled browser engine (guideline B4):
// `node web/benchmark.mjs <genre> <shape> [seeds]`, after `node web/build.mjs`.
//
// Generates seeds 0 to `seeds - 1` (default 20) at each level with the web
// app's limits and prints the same first columns as the native benchmark.
// Failures count as infinite time in the median.
import {readFile} from 'node:fs/promises';
import init,{request} from './dist/wasm/pencil_puzzles_web.js';

const [genre,shape,count='20']=process.argv.slice(2);
if(!genre||!shape)throw new Error('usage: node web/benchmark.mjs <genre> <shape> [seeds]');
await init({module_or_path:await readFile(new URL('./dist/wasm/pencil_puzzles_web_bg.wasm',import.meta.url))});
const seeds=Number(count);
console.log('shape,requested,success,median_ms');
for(const difficulty of ['easy','medium','hard','expert']){
  const times=[];
  for(let seed=0;seed<seeds;seed++){
    const start=performance.now();
    const result=JSON.parse(request(JSON.stringify({operation:'generate',genre,params:shape,difficulty,seed:String(seed)})));
    if(result.ok)times.push(performance.now()-start);
  }
  times.sort((a,b)=>a-b);
  const median=times[Math.floor(seeds/2)]??Infinity;
  console.log(`${shape},${difficulty},${times.length},${median.toFixed(1)}`);
}
