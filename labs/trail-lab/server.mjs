import http from 'node:http';
import {readFile} from 'node:fs/promises';
const files={
  '/':'index.html','/index.html':'index.html','/app.js':'app.js',
  '/engine.mjs':'engine.mjs','/style.css':'style.css',
  '/data/meta.json':'data/meta.json','/data/trace.bin':'data/trace.bin',
  '/data/terrain.bin':'data/terrain.bin','/data/trail.bin':'data/trail.bin',
};
const types={'.html':'text/html','.css':'text/css','.js':'text/javascript','.mjs':'text/javascript','.json':'application/json','.bin':'application/octet-stream'};
http.createServer(async(req,res)=>{
  const file=files[req.url?.split('?')[0]];
  if(!file){res.writeHead(404);return res.end('Not found');}
  try{
    const data=await readFile(new URL(file,import.meta.url));
    const ext=file.slice(file.lastIndexOf('.'));
    res.setHeader('Content-Type',types[ext]??'application/octet-stream');
    res.end(data);
  }catch{res.writeHead(500);res.end('Unable to read file — generate lab data first (see README.md)');}
}).listen(4175,'127.0.0.1',()=>console.log('Local: http://127.0.0.1:4175'));
