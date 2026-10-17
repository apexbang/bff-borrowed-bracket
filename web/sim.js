// 2D BFF soup core. Runs as a Web Worker in the page, and under node for
// seed scanning (see scan.mjs). Same rules as soup/src/bin/grid.rs.

const PROG = 64, TAPE = 128, MASK = 127, MAX_STEPS = 8192;
const LT = 60, GT = 62, LB = 123, RB = 125, PLUS = 43, MINUS = 45, DOT = 46, COMMA = 44, OPEN = 91, CLOSE = 93;

function sfc32(a, b, c, d) {
  return function () {
    a |= 0; b |= 0; c |= 0; d |= 0;
    const t = (((a + b) | 0) + d) | 0;
    d = (d + 1) | 0;
    a = b ^ (b >>> 9);
    b = (c + (c << 3)) | 0;
    c = (c << 21) | (c >>> 11);
    c = (c + t) | 0;
    return t >>> 0;
  };
}

function makeRng(seed) {
  const r = sfc32(0x9e3779b9, 0x243f6a88, 0xb7e15162, seed >>> 0);
  for (let i = 0; i < 20; i++) r();
  return r;
}

// Run one 128-byte tape in place; returns instructions executed.
function run(t) {
  let h0 = 0, h1 = 0, pc = 0, ops = 0;
  for (let step = 0; step < MAX_STEPS; step++) {
    switch (t[pc]) {
      case LT: h0 = (h0 - 1) & MASK; break;
      case GT: h0 = (h0 + 1) & MASK; break;
      case LB: h1 = (h1 - 1) & MASK; break;
      case RB: h1 = (h1 + 1) & MASK; break;
      case PLUS: t[h0] = (t[h0] + 1) & 255; break;
      case MINUS: t[h0] = (t[h0] - 1) & 255; break;
      case DOT: t[h1] = t[h0]; break;
      case COMMA: t[h0] = t[h1]; break;
      case OPEN:
        if (t[h0] === 0) {
          let d = 1;
          while (d > 0) {
            if (++pc >= TAPE) return ops + 1;
            if (t[pc] === OPEN) d++; else if (t[pc] === CLOSE) d--;
          }
        }
        break;
      case CLOSE:
        if (t[h0] !== 0) {
          let d = 1;
          while (d > 0) {
            if (--pc < 0) return ops + 1;
            if (t[pc] === CLOSE) d++; else if (t[pc] === OPEN) d--;
          }
        }
        break;
      default: ops--;
    }
    ops++;
    if (++pc >= TAPE) break;
  }
  return ops;
}

class Soup {
  constructor(w, h, seed, mutation) {
    this.w = w; this.h = h; this.n = w * h;
    this.mutation = mutation;
    this.rng = makeRng(seed);
    this.data = new Uint8Array(this.n * PROG);
    for (let i = 0; i < this.data.length; i += 4) {
      const r = this.rng();
      this.data[i] = r; this.data[i + 1] = r >>> 8; this.data[i + 2] = r >>> 16; this.data[i + 3] = r >>> 24;
    }
    this.order = new Uint32Array(this.n).map((_, i) => i);
    this.claimed = new Uint8Array(this.n);
    this.tape = new Uint8Array(TAPE);
    this.epoch = 0;
  }

  below(n) { return Math.floor((this.rng() / 4294967296) * n); }

  step() {
    const { w, h, n, order, claimed, data, tape } = this;
    for (let i = n - 1; i > 0; i--) {
      const j = this.below(i + 1);
      const t = order[i]; order[i] = order[j]; order[j] = t;
    }
    claimed.fill(0);
    let ops = 0, pairs = 0;
    for (let k = 0; k < n; k++) {
      const i = order[k];
      if (claimed[i]) continue;
      const r = this.rng();
      const dx = (r % 5) - 2, dy = ((r >>> 8) % 5) - 2;
      if (dx === 0 && dy === 0) continue;
      const x = (i % w) + dx, y = ((i / w) | 0) + dy;
      if (x < 0 || y < 0 || x >= w || y >= h) continue;
      const j = y * w + x;
      if (claimed[j]) continue;
      claimed[i] = claimed[j] = 1;
      tape.set(data.subarray(i * PROG, i * PROG + PROG), 0);
      tape.set(data.subarray(j * PROG, j * PROG + PROG), PROG);
      ops += run(tape);
      data.set(tape.subarray(0, PROG), i * PROG);
      data.set(tape.subarray(PROG), j * PROG);
      pairs++;
    }
    // Poisson-ish mutation count: mean is small enough that rounding a
    // uniform jitter around it is indistinguishable here.
    const mean = this.mutation * data.length;
    const k = Math.floor(mean + (this.rng() / 4294967296) * 2 * Math.sqrt(mean + 1) - Math.sqrt(mean + 1) + 0.5);
    for (let m = 0; m < k; m++) data[this.below(data.length)] = this.rng() & 255;
    this.epoch++;
    return pairs ? ops / pairs : 0;
  }
}

if (typeof module !== 'undefined') module.exports = { Soup, run, PROG, TAPE };
