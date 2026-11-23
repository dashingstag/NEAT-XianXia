import http from 'node:http';
import { readFile } from 'node:fs/promises';

// One entry point for both labs. /trail/* serves trail-lab, /cover/*
// serves path-cover-lab; both labs' relative data/ fetches resolve to
// the shared trail-lab generator output.
const roots = {
  '/trail/': '../trail-lab/',
  '/cover/': '../path-cover-lab/',
};
const types = {
  '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript',
  '.mjs': 'text/javascript', '.json': 'application/json',
  '.bin': 'application/octet-stream',
};

async function serve(rel, res) {
  try {
    const data = await readFile(new URL(rel, import.meta.url));
    res.setHeader('Content-Type',
      types[rel.slice(rel.lastIndexOf('.'))] ?? 'application/octet-stream');
    res.end(data);
  } catch {
    res.writeHead(500);
    res.end('Unable to read file — run the trail_lab generator first');
  }
}

http.createServer(async (req, res) => {
  const url = req.url?.split('?')[0] ?? '/';
  if (url === '/' || url === '/index.html') return serve('index.html', res);
  for (const [prefix, root] of Object.entries(roots)) {
    if (!url.startsWith(prefix)) continue;
    const rel = url.slice(prefix.length) || 'index.html';
    if (rel.startsWith('data/')) {
      return serve(`../trail-lab/data/${rel.slice(5)}`, res);
    }
    return serve(root + rel, res);
  }
  res.writeHead(404);
  res.end('Not found');
}).listen(4177, '127.0.0.1', () => console.log('Local: http://127.0.0.1:4177'));
