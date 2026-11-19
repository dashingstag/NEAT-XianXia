import {evolve,audit} from './engine.mjs';
self.onmessage=({data})=>{try{const result=data.kind==='audit'?audit(data.seed):evolve({...data.args,onGeneration:row=>self.postMessage({kind:'progress',row})});self.postMessage({kind:data.kind,result});}catch(e){self.postMessage({kind:'error',message:e.message});}};
