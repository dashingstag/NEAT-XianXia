import http from 'node:http';
import {readFile} from 'node:fs/promises';
const files={'/':'index.html','/index.html':'index.html','/app.js':'app.js','/engine.mjs':'engine.mjs','/economy.mjs':'economy.mjs','/style.css':'style.css'};
http.createServer(async(req,res)=>{const file=files[req.url?.split('?')[0]];if(!file){res.writeHead(404);return res.end('Not found');}try{res.setHeader('Content-Type',file.endsWith('.css')?'text/css':/\.(js|mjs)$/.test(file)?'text/javascript':'text/html');res.end(await readFile(new URL(file,import.meta.url)));}catch{res.writeHead(500);res.end('Unable to read file');}}).listen(4175,'127.0.0.1',()=>console.log('Village simulation: http://127.0.0.1:4175'));
