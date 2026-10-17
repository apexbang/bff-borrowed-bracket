// Does life take over when the browser sim starts from a crop of the 2D run?
const { Soup } = require('./sim.js');
const zlib = require('zlib');
const fs = require('fs');
const [file, seeds, epochs] = [process.argv[2], +process.argv[3], +process.argv[4]];
function hoe(d) {
  const c = new Float64Array(256);
  for (const b of d) c[b]++;
  let H = 0;
  for (const x of c) if (x) { const p = x / d.length; H -= p * Math.log2(p); }
  return H - zlib.deflateRawSync(d, { level: 6 }).length * 8 / d.length;
}
const crop = fs.readFileSync(file);
for (let seed = 1; seed <= seeds; seed++) {
  const s = new Soup(128, 72, seed, 0.00024);
  s.data.set(crop);
  let at = null;
  while (s.epoch < epochs) { s.step(); if (s.epoch % 50 === 0 && hoe(s.data) > 1) { at = s.epoch; break; } }
  console.log(file, 'seed', seed, 'life at', at);
}
