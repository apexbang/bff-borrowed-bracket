// Scan seeds for an early transition at browser scale.
// usage: node scan.js first last maxEpochs [w h]
const { Soup } = require('./sim.js');
const zlib = require('zlib');
const [first, last, maxE, w = 128, h = 72] = process.argv.slice(2).map(Number);

function hoe(d) {
  const c = new Float64Array(256);
  for (const b of d) c[b]++;
  let H = 0;
  for (const x of c) if (x) { const p = x / d.length; H -= p * Math.log2(p); }
  return H - zlib.deflateRawSync(d, { level: 6 }).length * 8 / d.length;
}

for (let seed = first; seed <= last; seed++) {
  const s = new Soup(w, h, seed, 0.00024);
  const t0 = Date.now();
  let found = null;
  while (s.epoch < maxE) {
    s.step();
    if (s.epoch % 100 === 0 && hoe(s.data) > 1.0) { found = s.epoch; break; }
  }
  console.log(`seed ${seed} transition ${found} after ${((Date.now() - t0) / 1000).toFixed(0)}s`);
}
