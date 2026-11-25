// Route checks for the combined lab server: node --test tests.mjs
// Starts server.mjs on a free port, so it never clashes with a running lab.
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const here = (rel) => new URL(rel, import.meta.url);
let proc;
let base;

before(async () => {
  // fileURLToPath, not URL.pathname: the latter yields /D:/... on Windows.
  proc = spawn(process.execPath, [fileURLToPath(here('server.mjs'))], {
    env: { ...process.env, PORT: '0' },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  base = await new Promise((resolve, reject) => {
    let out = '';
    proc.stdout.on('data', (chunk) => {
      out += chunk;
      const m = out.match(/Local: (http:\/\/127\.0\.0\.1:\d+)/);
      if (m) resolve(m[1]);
    });
    proc.on('exit', (code) => reject(new Error(`server exited early (${code})`)));
  });
});

after(() => proc.kill());

async function expectFile(route, file, type) {
  const res = await fetch(base + route);
  assert.equal(res.status, 200, `${route} status`);
  assert.equal(res.headers.get('content-type'), type, `${route} content type`);
  assert.equal(await res.text(), await readFile(here(file), 'utf8'), `${route} body is ${file}`);
}

test('the entry page links both labs', async () => {
  await expectFile('/', 'index.html', 'text/html');
  await expectFile('/index.html', 'index.html', 'text/html');
  const page = await (await fetch(base + '/')).text();
  assert.match(page, /href="\/cover\/"/);
  assert.match(page, /href="\/trail\/"/);
});

test('each lab route maps to its own directory', async () => {
  await expectFile('/cover/', '../path-cover-lab/index.html', 'text/html');
  await expectFile('/cover/engine.mjs', '../path-cover-lab/engine.mjs', 'text/javascript');
  await expectFile('/cover/app.js', '../path-cover-lab/app.js', 'text/javascript');
  await expectFile('/trail/', '../trail-lab/index.html', 'text/html');
  await expectFile('/trail/engine.mjs', '../trail-lab/engine.mjs', 'text/javascript');
  await expectFile('/trail/style.css', '../trail-lab/style.css', 'text/css');
});

test('both labs share the trail-lab data directory', async () => {
  // data/ holds generated output; its tracked .gitignore stands in as a fixture.
  for (const lab of ['cover', 'trail']) {
    await expectFile(`/${lab}/data/.gitignore`, '../trail-lab/data/.gitignore',
      'application/octet-stream');
  }
});

test('unknown routes and missing files fail clearly', async () => {
  assert.equal((await fetch(base + '/nowhere')).status, 404);
  const missing = await fetch(base + '/trail/data/not-generated.bin');
  assert.equal(missing.status, 500);
  assert.match(await missing.text(), /run the trail_lab generator first/);
});
