import init, { request } from './wasm/pencil_puzzles_web.js';
const ready = init();
self.onmessage = async ({data}) => {
  try { await ready; self.postMessage({id:data.id, ...JSON.parse(request(JSON.stringify(data.request)))}); }
  catch (error) { self.postMessage({id:data.id,ok:false,error:error.message || 'Could not start the puzzle engine.'}); }
};
