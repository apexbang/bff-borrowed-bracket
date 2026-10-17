# Journal

## Summary (read this first)

**What I did:** reproduced "Computational Life" (Agüera y Arcas et al.,
2024). Random 64-byte programs in a self-modifying Brainfuck dialect pair
up and run on a shared tape. With no fitness function, self-replicators
appear and take over. Then I tried to catch the exact moment life begins,
and found out how it happens.

**Main findings** (details below, in time order):

1. **Reproduced.** The well-mixed soup came alive in 2 of 8 seeds within 16k
   epochs (paper: ~40%). The 2D soup forms visible replicator fronts.
   Replicators always copy themselves *mirrored*, walking one head backwards
   from byte 0 into the partner. They survive by being near-palindromes,
   and in the well-mixed soup every species shows up as a mirror-image twin
   pair with near-equal counts (263/261, 201/198, …).
2. **How life starts: a borrowed bracket.** Births are deterministic and
   replayable, so I wrote forensics tools that find the first replicator and
   replay the single interaction that made it. In **45 of 49 births** (60 2D
   runs) the parent was a *latent* copier completed by its partner. In 42 of
   those, the parent's copy loop had no closing `]` and closed on a `]` in
   the partner's half of the tape. The copy it wrote had the bracket built
   in. Only 2 births came from a lucky mutation.
3. **Why brackets are lying around: armour.** In the lifeless soup, 10-14% of
   programs start with `]` (29x more than at any other position). A leading
   `]` makes a program halt instantly when it runs first, so armoured
   programs don't wreck themselves and persist. They're the commonest bracket
   donors (byte 0 in 17 of 42 seam births).
4. **Confirmed by intervention (day 2).** With loops forbidden from closing
   across the seam: **0 births in 3M epochs** (control: 1.75 per 100k).
   Stripping armour each epoch: births fall **3x** (0.58 per 100k). Sealing
   also impoverishes pre-life chemistry, so a second test sealed *already
   mature* soups for short windows: **0 births vs 9** from identical unsealed
   starts, 4 of those within the first 400 epochs. The block works at the
   moment of birth. See "Day 2" at the end.
5. **It generalises (day 2).** In the paper's other BFF variant, where
   bytes 0-1 aim the heads, replicators become direct (not mirrored)
   copiers, but all 7 traced births were still completed by the partner
   (5 through a seam loop).
6. **Noise alone doesn't do it.** 0 heritable replicators in 10^9 uniform
   random programs (5.7 per million make one dead-end mirror copy).
7. Births are rare and steady: ~1e-10 per program-epoch. Half of new
   lineages die within a few hundred epochs. A pre-registered check (5/20
   seeds alive by 40k epochs, predicted 5-6) killed a batch effect I had
   started to believe in.

I also got two things wrong along the way and corrected them in place: a
contaminated "64x composition enrichment" number (03:00 entry), and copier
tests that missed copies shifted by a few bytes (05:30 entry).

**Look at:**
- `ESSAY.md`: the whole story in one read (day 2).
- `web/soup.html`: open in a browser. A live 2D soup that starts at the real
  birth moment from the overnight run (the first replicator is ringed), plus
  a step-by-step replay of the birth interaction. Self-contained, works from
  disk. I did *not* publish it as an artifact, since publishing to your
  claude.ai account felt too close to "acting as the owner"; happy to if you
  want.
- `media/takeover_2d_seed1.mp4` (7 s) and `media/takeover_frames.png`.
- `soup/forensics/`: replayed birth events and the full births table.

**Run:** see README.md. In short: `cd soup && cargo build --release`, then
`./target/release/births --seed 25 --threads 8` traces a birth in about a
minute (seed 25 comes alive at epoch 8,200).

**What I'd do next:**
- ~~Intervention tests~~ done on day 2, including sealing mature soups.
- The same forensics on the well-mixed soup at scale (needs pair logging,
  which `soup --pairs-at` already supports).
- Forth turned out to be no test: its `0x0C` byte is a complete one-byte
  replicator. A copier test based on essential bytes would close the
  heads-variant blind spot (core-only copiers).

---


## 00:20 — picking a project

The directory was empty, so I picked something I've wanted to try: reproduce
the "Computational Life" experiment (Agüera y Arcas, Alakuijala, Evans, Laurie,
Mordvintsev, Niklasson, Randazzo, Versari, 2024; arXiv:2406.19108).

The setup: 131,072 random 64-byte strings. Each epoch, shuffle them into pairs,
glue each pair into one 128-byte tape and run it as a program in "BFF", a
Brainfuck dialect with two data heads where code and data live on the same
tape. So a program can read and overwrite itself *and* its partner. Then split
the tape back in two. Add a pinch of random mutation. Repeat.

There is no fitness function and no selection. Still, the paper reports that
in ~40% of runs, within 16k epochs, self-replicating programs appear out of
the noise and take over the soup. You can see it as a sudden jump in
"high-order entropy" (Shannon entropy minus compressed size per byte): random
bytes score ~0, a soup full of copies of a few things scores high.

Implementation: Rust crate in `soup/`. Semantics follow the reference
implementation (cubff): heads start at 0, wrap mod 128, unmatched bracket
halts, 8192-byte-read budget.

## 00:30 — tools

- `soup` (main binary): the well-mixed soup of the paper. 2^17 programs.
  Logs Shannon entropy, brotli-q2 compressed size and their difference
  (high-order entropy, HOE) every 64 epochs.
- `grid`: the paper's 2D variant: 240x135 programs, each only pairs with a
  neighbour within 2 cells. Renders each program as an 8x8 tile of coloured
  instructions and pipes frames into ffmpeg.
- `dissect`: takes a snapshot apart: commonest programs, how faithfully each
  copies itself into a random partner, which bytes are essential (knock each
  out and see if copying breaks), and an execution trace.
- `search`: samples random programs and counts copiers.

Sanity check: a hand-written palindromic replicator `[{.>]` + 54 junk +
`]>.{[` copies itself 100%, and the knockout analysis finds the core `[{.>`.
It doesn't flag the `]`: knock it out and the spare `]` near the end closes
the loop instead.

## 00:45 — first life, in 2D

2D seed 1: high-order entropy sat around 0.15 for 8000 epochs, then went
0.2 → 2.6 → 4.0 in about 700 epochs. In the video
(`soup/videos/grid_s1.mp4`, not committed, 68 MB) you can watch a uniform
patch appear in the upper right and sweep the grid as a front. It's like
watching a crystal grow, except the crystal is a program.

The replicator it found:

    ··[·······<,··}·]·········{·····}········]·········}··,<······[·
      ^       ^^  ^          (essential bytes: [ < , })

Loop `[ < , } ]`: `<` walks head0 *backwards* (0 wraps to 127, into the
partner), `,` copies the byte under head1 there, `}` walks head1 forward
through itself. So it writes its **mirror image** into the partner. That
alone would be a dead end (the mirror of `[<,}]` doesn't loop). Look at the
tail though: `}··,<······[·` is the head reversed. The program is roughly a
palindrome, so its mirror image is roughly itself. Evolution found the
palindrome fix. My own hand-written replicator needed the same trick.

After the takeover HOE fell back from 4 to 1.5. The soup is now a cloud of
damaged variants of that one family. Many have lost their closing `]` but
persist anyway. My guess: the tape is both programs glued together, so a
loop that opens in one can close on a `]` in the partner. In a soup of
relatives that almost always works; against random noise (my test) it
doesn't. Replication becomes partly a group activity.

## 00:50 — how rare are replicators in pure noise?

Brute force over uniform random 64-byte strings: about **4.75 in a million**
copy ≥90% of themselves into a random partner. That's much more common than
I expected. The loop's instructions don't need to be adjacent, since junk
bytes in between are no-ops, so `[ … { … . … > … ]` scattered anywhere works.
So the initial soup of 131k programs already contains a copier or two.

But none of the 19 copiers in a 4M sample is *heritable*: a lineage
followed for 4 generations falls apart, because the copy is a mirror image
whose loop no longer reads as a loop. Palindromic ones are what you need,
and those are far rarer. Running 1e9 samples to get a number.

## 01:20 — life in the well-mixed soup, and mirror twins

Well-mixed seed 1 came alive at epoch ~5700. HOE went 0.1 → 1.2 in ~130
epochs and kept climbing. That's much faster than the 2D front (~700
epochs), as the paper says: in a well-mixed soup a replicator meets
everyone at once, so takeover is roughly logarithmic in soup size.

The winner, at the moment of transition:

    ······[·····{·.>]·<··········<·<]·]<·<<·········<·]>.·{·····[···
          ^     ^ ^^^      (essential: [ { . > ])

That's exactly the loop I'd hand-written as a sanity check: `{` walks
head1 backwards into the partner, `.` writes there, `>` advances head0
through itself. Again with a mirrored tail `]>.·{·····[`.

The best part: **every species comes in a mirror pair with near-equal
counts**. Top programs at transition: 263 and 261 copies, reverses of each
other; then 201/198; then 160/159. Each copy flips orientation, so each
lineage alternates between two forms every generation, like a
two-stroke engine.

So the two runs found the two mirror-copy solutions BFF offers:

| run          | loop      | who walks backwards | copy instruction |
|--------------|-----------|---------------------|------------------|
| 2D seed 1    | `[<,}]`   | head0 (the reader)  | `,` pulls        |
| mixed seed 1 | `[{.>]`   | head1 (the writer)  | `.` pushes       |

Neither found a forward copier (one that moves head1 to byte 64 first and
copies in order). That would need ~64 `}` instructions to get head1 there,
which is hopeless. Walking backwards from 0 wraps straight into the
partner for free. The geometry of the tape picks the solution.

## 01:35 — what pre-life chemistry does to the bytes

Before any replicator exists, Shannon entropy slides from 8.00 to ~7.55
bits/byte. So the soup is changing composition while nothing is copying
itself yet. Byte counts in seed 1's soup right at the transition (93% of it
still pre-life):

- instruction bytes: **17.2% of all bytes vs 3.9% for uniform noise (4.4x)**
- `<` 11.8x, `>` 7.0x, `[` 6.3x, `{` 5.9x, `.` 5.4x, `,` 4.6x
- zero bytes: 0.34x (depleted)
- several enriched non-instructions sit one or two values away from an
  instruction (`:` and `=` next to `<`). My guess is these are instructions
  that got hit by `+`/`-`.

Prediction: a copier needs ~5 particular instructions in the right order, so
pre-life enrichment should raise its odds by orders of magnitude, even with
no replicator around. To test, sample i.i.d. bytes with the soup's histogram
(`search --like`) and compare with uniform.

## 02:10 — the birth of a replicator, caught in the act

The 2D run is deterministic, so I replayed it with snapshots and ran a census
(every tile tested against random partners) to find the first moment a
heritable copier exists:

    epoch 7800 … 7903:  0 copiers among 32,400 tiles
    epoch 7904:         1  ← tile (177,43)
    epoch 7910:         3
    epoch 7950:        25
    epoch 8000:       198  (front visible in the video)

A small forensics tool (`soup/src/bin/origin.rs`) takes snapshots e-1 and e,
tries every neighbour pairing of the tile in both orders, and reports which
one reproduces the new state. Exactly one does, matching **64/64 bytes of
the tile and 64/64 of the partner**:

    epoch 7904
    parent (176,41) [·[·[·[·[·[···,·························<···,·········}·}·······
    tile   (177,43) ]························{···························[····00000·
                     ↓ 1,740 instructions
    tile becomes    [[[·······<,··}·]······························]·}··,<·······[[[

The parent is **not** a replicator by itself: its loop `[ … , … < … , … } }`
has no closing `]`. But the tile it was paired with *starts* with `]`. The
two are glued into one tape, so the loop closes across the seam. It then
walks head0 backwards into the tile and writes a mirrored, cleaned-up copy:
a complete palindromic replicator that carries its own `]`. Life began when
an incomplete copier borrowed a bracket from a stranger.

One step further back (epoch 7903), the parent itself had just been formed
the same way. The program at (176,41) paired with (175,39), which also
starts with `]`, and the loop rewrote it into the `[·[·[·[·[·[` form. So
this lineage got by on borrowed brackets for at least two generations before
it produced something self-sufficient.

The rare ingredient isn't the partner: 11% of tiles start with `]`, and
1,245 tiles carry a similar `<`…`}}` loop skeleton. It's the particular
parent program, whose op skeleton `[[[[[[,<,}}` occurs exactly once in the
soup.

## 02:40 — noise alone almost never makes life

The 1e9-sample search over uniform random programs finished:

- 5,731 copiers (5.7 per million) copy ≥90% of themselves into a partner
- **0 heritable**. Not one lineage survives 4 generations. 95% upper
  bound: ~3 per billion.

With bytes drawn i.i.d. from the byte histogram of seed 1's soup at the
transition, a quick 2M sample gave 730 copiers (3.7e-4, **64x** uniform) and
1 heritable (5e-7, at least **150x** uniform).

Scale check: the well-mixed soup runs 131k programs x ~6k epochs ≈ 8e8
program evaluations before its transition. Pure noise at <3e-9 would expect
fewer than ~2 heritable replicators in that whole time, and most of those
would die young (see below). So life here isn't a lucky draw from noise. The
pre-life soup first changes its own composition (4.4x more instruction
bytes), and that shift is what makes replicators findable. The census replay
(running) will check this against real snapshots instead of a histogram.

Browser-size seed scan (128x72, JS port; cross-checked against the Rust
interpreter on 20k tapes, identical): 1 of 20 seeds came alive within 10k
epochs (seed 10, at 2,600). The 240x135 grid came alive at 8,200. A smaller
soup gives fewer chances per epoch.

Early life is fragile. Starting the browser sim from a crop of the 2D run
at epoch 7,904 (one replicator, just born), it took over in 1 of 6 runs with
different random pairings. From the crop at 7,910 (three replicators) it took
over in 5 of 6.

## 03:00 — correction: the census undoes my composition story

I replayed well-mixed seed 1 with snapshots every 640 epochs. For each one,
**census** = test every real program in the soup, and **composition** =
2M programs drawn i.i.d. from that snapshot's byte histogram:

| epoch | op fraction | census: copiers / heritable | composition: copiers per program / heritable in 2M |
|------:|------:|------:|------:|
|   640 | 6.0%  | 0 / 0 | 2.6e-5 / 0 |
|  1280 | 8.6%  | 0 / 0 | 7.1e-5 / 0 |
|  1920 | 10.7% | 1 / 0 | 9.3e-5 / 0 |
|  2560 | 12.6% | 1 / 0 | 8.1e-5 / 0 |
|  3200 | 13.4% | 0 / 0 | 5.5e-5 / 0 |
|  3840 | 14.2% | 1 / 0 | 5.6e-5 / 0 |
|  4480 | 14.5% | 0 / 0 | 3.4e-5 / 0 |
|  5120 | 14.5% | 1 / 0 | 4.5e-5 / 0 |
|  5760 | 17.2% | **9,591 / 9,134** | 3.7e-4 / 1 |

Three corrections to the 02:40 entry:

1. **The "64x / 150x" numbers were contaminated.** They came from the
   epoch-5760 snapshot, which already holds ~9k real replicators that
   skew the byte histogram. With clean pre-life snapshots, composition
   raises copier odds only 5-16x over uniform. It peaks around epoch 1920
   and then *falls* while the instruction fraction keeps rising. It
   produces no heritable copiers in 2M samples at any snapshot.
2. **The real soup has fewer copiers than its own composition predicts.**
   Composition says 3-12 copiers should exist at any moment; the census
   finds 0-1. The extra instruction bytes are clumped into degenerate junk
   (`[[[[[[…`, `<<<<<<…`), not scattered the way i.i.d. sampling assumes.
3. So "composition shift makes replicators findable" is **not supported**.
   What I can still stand behind: noise essentially never yields heritable
   replicators (0 in 1e9), and the one birth I traced exactly (2D, 02:10)
   came from an *interaction*: a non-copier closing its loop on a
   partner's `]` and writing out a complete replicator. That fits the
   paper's own conclusion that interaction and self-modification matter more
   than random generation.

Lesson for me: I should have waited for clean snapshots before writing a
headline number.

## 03:30 — a second birth, same trick

Same forensics for the well-mixed soup (seed 1). Partners are random here,
so I logged each epoch's pairing (`--pairs-at`) and replayed single
interactions (`origin_mixed`). Records in `soup/forensics/`.

Census per epoch: a copying lineage exists from epoch 5557, and my test
first calls one "heritable" at 5565. Tracing back from that one showed my
heritability test was too strict. Each ancestor copies itself, but the copy
is shifted one byte (a stray `}` breaks the palindrome), so the lineage
creeps along the tape a byte every couple of generations. Position-wise
comparison after 4 generations calls that "not heritable". The real birth is
earlier, at **epoch 5560**, in one interaction of 3,054 instructions:

    first  (65597) ········}······,········[················[······{·······.··,>··>
    second (18338) ]·<··<<·<············<··········<·····<·<···············<····.[·
                    ↓
    second becomes }···[·····{·.>]·<·········<<·<]·]<·<<·········<·]>.·{·····[···

The first program holds the parts of a copy loop, `[ … { … . … >`, and
**no `]`**. Its partner **starts with `]`**. The loop closes across the seam
and mirror-writes a complete, near-palindromic replicator over the partner.

**Two births traced, two borrowed brackets**, in different setups (2D vs
well-mixed) with different loops (`[<,}]` vs `[{.>]`). Neither parent was a
replicator. Each was an *incomplete* copier that only worked with a partner
whose first byte happened to be `]`. The child got the bracket built in.
That's my favourite result of the night: the first replicator is a joint
product of two non-replicators, which is about as literal as "life from
interaction" gets.

Caveats: n=2. And the "heritable" counts in the census tables above
undercount drifting lineages, so the 03:00 table's census column is a lower
bound. The 0-in-1e9 uniform result uses the same strict test. It may miss
drifting replicators in noise, though a drifting one would still need a
working loop that survives its own mirror image, so I doubt it changes much.

Batch result: 2 of 8 well-mixed seeds came alive within 16k epochs (seed 1
at 5760, seed 5 at 14464). The paper says ~40%; n=8 is consistent with that.

## 04:15 — armour, and why the bracket is lying around

Automated the birth forensics (`soup/src/bin/births.rs`). It runs a 2D soup,
keeps the last 400 epochs (soup + pairings) in RAM, and when HOE passes 1 it
finds the first heritable copier. Then it walks the lineage back through
interactions until neither the tile's previous self nor the partner that
rewrote it was a copier. It replays that interaction with an instrumented
interpreter (`run_seam`) that records any loop whose `]` and `[` sit in
different programs. On seed 1 it reproduces the hand-traced birth exactly:
`]@64->[@10 x63`, i.e. the partner's first byte closed the parent's loop 63
times.

Then 12 more 2D seeds × 40k epochs: **none came alive.** I first suspected a
closing window, with pre-life drifting into something hostile. But seed 2's
composition is flat from ~10k to 40k epochs (11.7% instructions). Life is
a rare event at a steady rate. Pooling every run tonight:

| setup | births / program-epochs | rate |
|---|---|---|
| well-mixed 131k (8 seeds x 16k) | 2 / ~1.5e10 | ~1e-10 |
| 2D 240x135 (13 seeds) | 1 / ~1.6e10 | ~6e-11 |
| 2D 128x72 JS (20 seeds x 10k) | 1 / ~1.8e9 | ~5e-10 |

Same order of magnitude everywhere. 2D seed 1, the first one I tried, was
lucky.

**Armour.** In the lifeless soup, 10% of programs start with `]`, though `]`
is only 0.5% of bytes. Per position, `]` is **29x more common at byte 0**
than anywhere else, and `[` is slightly depleted there. Reason: a program
whose first byte is `]` halts instantly when it runs as the first half (`]`
sees a nonzero byte, scans back for `[`, falls off the start). Armoured
programs don't damage themselves, so they persist. Pre-life selection,
with nothing replicating yet.

That ties the night together. The armour leaves loose `]`s at exactly the
spot where a first-half program's loop can reach them: byte 64, right
across the seam.

**Latent copiers.** New census flag `--latent` counts programs that copy
themselves (≥90%) *only* when the partner starts with `]`:

| soup | latent | standalone copiers |
|---|---|---|
| 2D seed 2, 16 lifeless snapshots x 32,400 | 20 | 1 |
| mixed seed 1, 8 pre-life snapshots x 131,072 | 35 | 4 |

Latent copiers outnumber standalone ones roughly 9-20x. Only ~1 in 20
interactions offers a suitable partner (10% `]`-led x must be the first
half), so per interaction the two routes come out similar. The bracket route
is a big contributor but this count doesn't show it dominating. A batch of 27
seeds x 150k epochs is running to collect more births and see which route
they take.

## 05:30 — births, plural

Reworked `births` to census every 100 epochs and trace each new heritable
lineage as soon as it appears, whether or not it later takes over. Also
fixed a recurring flaw: copies are often shifted by a byte or two (the
loop's first pass writes in place), and my position-by-position comparison
called those "not copiers". The tracer then stopped on an older drifting
lineage and reported a false "birth without a seam loop" (seed 15, and seed
14 likewise). `similarity` now takes the best alignment within ±4 bytes.
Seed 1's birth is unchanged under the new test.

Births traced so far (2D unless noted; batch still running):

| run | birth epoch | mechanism | partner supplies | fate |
|---|---|---|---|---|
| 2D seed 1 | 7,904 | seam loop `]@64->[@10` x63 | `]` at its byte 0 | took over (8,200) |
| mixed seed 1 | 5,560 | seam loop (by hand) | `]` at its byte 0 | took over (5,760) |
| seed 14 | 68,544 | seam loop `]@67->[@50` x59 | `]` at byte 3 | took over (69,200) |
| seed 15 | 30,300 | seam loop `]@74->[@22` x54 | `]` at byte 10 | took over (31,100) |
| seed 16 | 73,091 | point mutation | — | died within ~110 epochs |
| seed 17 | 11,179 | seam loop `]@64->[@38` x64 | `]` at byte 0 | died within ~120 |
| seed 17 | 58,298 | seam loop `]@74->[@36` x52 | `]` at byte 10 | died within ~100 |
| seed 19 | 16,413 | seam loop `]@76->[@34` x53 | `]` at byte 12 | died within ~190 |
| seed 18 | 101,166 | **lock and key**, no seam loop | a zero at byte 126 | took over (101,400) |

Seed 18 is a different trick. The parent starts `<0········0<]`: two `<`
put head0 on byte 126, and `]` checks it. If it's nonzero, `]` scans back
for a `[`, finds none, and halts. So the program is locked unless its
partner's byte 126 is zero. Tested in node: 0% copies against random
nonzero partners (halts after 3 instructions), 100% with byte 126 zeroed
(262 instructions, exactly as in the birth). The child it writes has a zero
at byte 126 as well, so the lineage carries its own key.

So far: **every interaction birth was a latent copier completed by its
partner**, with a `]` in 7 cases and a zero byte in 1. None was a standalone
copier that formed out of nothing. Half of the births die within ~200
epochs; early life is fragile.

## 06:30 — the births batch, and a pre-registered check

Seeds 23-40 (18 seeds x 150k epochs, ±8 window): 26 births traced.
**22 via a seam loop** (a `]` in the partner closing a loop opened in the
parent), 15 of which took over. 2 "mutation" births are junk-byte changes on a
tile that was probably already a copier (tracer bug, now fixed, re-runs
pending), and 2 no-seam births still need a look. Two genuine mutation births
turned up in the re-runs (seeds 16 and 22). Both are the same event: a
mutation **deleted a premature `]`**, so the loop now runs on to a later `]`
and takes in the `<` and `}` it was missing. Both died young.

Time to life looked different between batches: seeds 1-22 gave 6 takeovers
in ~1.48M seed-epochs, seeds 23-40 gave 16 in ~1.09M (p ≈ 0.002 under a
common rate). My hypothesis was seed-dependent pre-life chemistry, so I
snapshotted all 39 seeds at epoch 5000 and measured armour (`]`-led
programs), instruction fraction, `]` fraction and latent copiers. **No
signal**: every seed looks alike (armour 12.8-14.6%), and nothing correlates
with time to life (Spearman |ρ| ≤ 0.3, mostly ~0). The simulation code is the
same in every batch (seed 1 reproduces exactly), and I picked the split after
seeing the data, so that p-value means little.

**Pre-registered prediction**, written before the results: with the pooled
rate (mean time to life ~120k epochs, P(alive by 40k) ≈ 0.28), fresh seeds
41-60 run for 40k epochs should give about **5-6 of 20** transitions. 0-2
would point to a real seed or batch effect I haven't found; ~9+ would point
the other way.

**Result: 5 of 20** fresh seeds came alive within 40k epochs (at 6,700,
6,800, 11,100, 38,200, 39,800). That's on the prediction, so the batch
difference was chance plus my after-the-fact split. No evidence that birth
rates depend on the seed. All **8 of 8** births in these fresh seeds came
via a seam loop.

The two no-seam births from seeds 35/36: seed 36's parent ran its own code
and one byte changed, a `<` copied in from the partner's tail that completed
its loop. So the partner supplied the missing piece as *data* this time.
Seed 35's change touched only junk-byte values (the program reads the same
before and after), so it's most likely noise in the 3-trial copier test on a
borderline copier.

## 07:15 — final tally: 49 births

All 2D seeds 1-60 traced with the final `births` binary (full table:
`soup/forensics/births_table.md`). 32 seeds produced at least one heritable
lineage; **49 births** in total.

| how the first copier came to be | births | took over |
|---|---|---|
| **seam loop**: the parent's copy loop closes on a `]` in its partner | **42** | 25 |
| lock and key: the parent runs only if its partner has a zero at byte 126 | 1 | 1 |
| the partner's byte (a `<`, a zero) copied in completes the parent | 2 | 1 |
| mutation deleting a premature `]` so the loop takes in more code | 2 | 0 |
| ambiguous (copier-test noise / inside an existing lineage) | 2 | 0 |

So **45 of 49 births (92%) were a latent copier completed by its
partner**, and 42 of those used a borrowed bracket. Where the borrowed `]`
sat in the partner: **byte 0 in 17 cases**, bytes 1-6 in 14 more. That fits
the armour story. Armoured programs (leading `]`) are 10-14% of a
pre-life soup, and they're the commonest bracket donors.

Only ~55% of births go on to take over; the rest die within ~100-200
epochs.

What I'd claim from tonight: in BFF, replicators don't usually appear as a
lucky complete program. They appear when one of the soup's many *latent*
copiers (loops missing their closing bracket, or gated on a byte) meets a
partner that supplies the missing piece. The copy it writes has the piece
built in. The pre-life soup helps along by making those bracket donors
common for an unrelated reason: a leading `]` protects a program from
running itself.

## Day 2, 09:00 — intervention: does blocking the borrowed bracket stop life?

Everything so far is observational: I watched births and saw borrowed
brackets. If that's the mechanism, changing the world to block it should
make births rarer. Two arms, each 20 fresh 2D seeds x 150k epochs (3M
epochs), compared with the control rate from last night: **1.75 births per
100k epochs** (49 in 2.79M, seeds 1 and 14-60) and 0.97 takeovers per 100k.

- **Sealed seam** (`births --sealed`): bracket matching stops at the
  boundary between the two programs, so a loop can't open in one and close
  in the other. Replicators that carry their own `]` are unaffected (unit
  test: under the sealed rule, last night's seed-1 birth interaction fails,
  and the replicator it produced still copies itself).
- **No armour** (`births --strip-armour`): every epoch, any program whose
  first byte is `]` gets that byte replaced. The interpreter is unchanged;
  this only removes the commonest bracket donors.

**Predictions, written before running:**
- Sealed: births fall to **under 0.6 per 100k** (fewer than ~18 in 3M
  epochs). If only the 7 non-seam routes of 49 survived, ~0.25 per 100k
  (~7 births). A result near 1.75 would mean other routes simply take over
  and the bracket story is about *which* route, not *whether*.
- No armour: a smaller drop, to **0.8-1.4 per 100k**. 17 of 42 seam births
  borrowed a byte-0 `]`, and the other 25 used `]`s further in, which
  stripping doesn't touch.

### Result

| arm | epochs | births | via seam | births /100k (95% CI) | takeovers /100k (95% CI) |
|---|---|---|---|---|---|
| control | 2,792,800 | 49 | 42 | 1.75 (1.30-2.32) | 0.97 (0.64-1.41) |
| **sealed seam** | 3,000,000 | **0** | 0 | **0.00 (0.00-0.12)** | 0.00 (0.00-0.12) |
| **no armour** | 2,565,800 | 15 | 14 | **0.58 (0.33-0.96)** | 0.27 (0.11-0.56) |

**Sealed: prediction met, more strongly than predicted.** Not one birth
in 3M epochs, at least ~14x rarer than control. I'd guessed the non-seam
routes would survive (~0.25 per 100k); they didn't either.

There's a confound I found while the arms ran. Sealing changes pre-life
chemistry a lot. At epoch 10k (seed 301), instruction bytes are 7.1% sealed
vs 11.7% control, and armour is 26% vs 10%. Cross-seam loops evidently do
much of the work of spreading instructions through pre-life. So the sealed
result says "no cross-seam loops, no life", but it can't separate *blocked
bracket-borrowing at the birth* from *a poorer soup beforehand*. Both are
the seam, though.

**No armour: prediction missed. The effect was bigger than I said.** I
predicted 0.8-1.4 and got 0.58, a 3x drop (conditional binomial vs control,
p ≈ 5e-5). Chemistry stays close to control here (10.7% instructions), so
this is the cleaner of the two tests. Stripping byte-0 armour cost more than
the 17 of 42 byte-0 donors suggested. My guess is that armour does more than
donate a bracket: an armoured program never runs as the first half, so it
survives intact and keeps its `]`s, the ones further in included, available
to its neighbours. Births still came almost entirely via seam loops (14 of
15). The borrowed `]` moved to **partner byte 1 in 5 of 14**: a program
starting `x]` with `x` a no-op is still armoured, and byte-1 `]`s did rise
a little (2.8% → 3.7%) without making up the loss.

So the causal story holds up under intervention: take away the cross-seam
loop and life doesn't start; take away the commonest donors and life
starts three times less often.

## Day 2, 11:00 — separating the seam's two roles

The sealed arm confounds two things: the cross-seam loop at the moment of
birth, and the seam's role in building pre-life chemistry. To pull them
apart: matured 10 soups (seeds 401-410) to epoch 10k under normal rules (all
lifeless, 11.1-11.9% instructions), then ran 5,000-epoch windows from each
snapshot with different random streams. 24 windows per snapshot are
**sealed from the first epoch** and 24 are unsealed controls. The windows are
short so the chemistry has little time to drift (I log instruction
fraction at the end of each window to check).

**Prediction, written before running:** control windows should give ~21
births (1.75 per 100k x 1.2M epochs). If the seam acts at the moment of
birth, sealed windows give close to 0 even though they start with mature
chemistry. If it acts only through chemistry, sealed windows start near the
control rate and fall off later.

### Result

| windows (5k epochs from mature snapshots) | epochs | births | births /100k (95% CI) | instr. at window end |
|---|---|---|---|---|
| unsealed (control) | 1,192,800 | 9 | 0.75 (0.35-1.43) | 11.7% |
| sealed from epoch 1 | 1,200,000 | **0** | 0.00 (0.00-0.31) | 9.3% |

- With identical starting soups and equal exposure, 9 vs 0 (if sealing
  changed nothing, all 9 landing in control has probability 0.5^9 ≈ 0.002).
- **4 of the 9 control births came within the first 400 epochs** of their
  window, before sealed chemistry could drift much. These are latent
  copier families already in the snapshots finding a partner. The sealed
  windows started from the same snapshots, with the same latent families,
  and produced nothing. Births came from 6 of the 10 snapshots.
- So the seam's role at the moment of birth is real and on its own enough
  to stop life. The chemistry route may matter too: sealed soups do drift
  (11.6% → 9.3% instructions over 5k epochs), but this test doesn't need it.

My prediction for the control count was off: ~21 expected, 9 observed. The
rate after the first 400 epochs was well below the long-run average (5 births
in ~1.1M epochs). Windows cut from 10 snapshots are correlated, and the
long-run rate may include slow processes that 5k-epoch windows don't
capture, such as latent lineages that build up over many generations before
a birth. I don't have a better explanation. The sealed/control contrast
doesn't depend on it.

**Where the bracket story stands now:**
1. Observation: 42 of 49 births close a loop across the seam (+3 more
   completed by the partner some other way).
2. Intervention, whole run: sealing the seam → 0 births in 3M epochs.
3. Intervention, at the moment of birth only: sealing a mature soup → 0
   births where identical unsealed soups give 9, 4 of them right away.
4. Donor supply: stripping armour → 3x fewer births.

## Day 2, 12:30 — is the borrowed bracket general? The heads variant

I'd planned to try the paper's Forth soup next, but reading the reference
code ruled it out as a test. In `forthtrivial` the byte `0x0C` (Copy0) at
position 0 copies *itself* into the partner: a complete one-byte
replicator. There's no latent copier to complete, so it can't say anything
about partner completion.

A better test is the paper's other BFF variant (cubff `bff`, vs
`bff_noheads` which I've used so far). Bytes 0 and 1 of the tape set head0
and head1, and execution starts at byte 2. A program can aim head1 straight
into its partner, so a *forward* copier no longer needs the wrap-around
mirror trick that forced palindromes (unit test: `[128, 64]` + `[.>}]`
copies itself exactly with heads on and not at all with heads off).

**Predictions, written before running:**
- Replicators will be mostly forward copiers (direct copies), not mirror
  palindromes.
- If partner completion is a general route to life, seam loops still
  dominate births (say over half). If the borrowed bracket was a quirk of
  the mirror geometry, they won't.

### Result

20 seeds x up to 100k epochs (1.43M epochs), copier threshold lowered to
60% (see below): 9 takeovers (0.63 per 100k, similar to the default
variant's 0.97), 7 traced births.

| seed | birth | mechanism | fate |
|---|---|---|---|
| 502 | 11,048 | seam loop, `]` at partner byte 35 | took over |
| 504 | 39,007 | partner's code wrote 2 bytes (`,` `,`) completing the tile's loop | took over |
| 506 | 50,786 | seam loop, `]` at partner byte 16 | took over |
| 510 | 13,149 | partner edit, 1 byte | took over |
| 515 | 85,285 | seam loop, `]` at partner byte 60 | took over |
| 517 | 48,635 | seam loop, `]` at partner byte 16 | took over |
| 518 | 48,665 | seam loop, `]` at partner byte 10 | took over |

**Prediction 2 met: all 7 births were completed by the partner, 5 of them
through a seam loop.** Partner completion survives a change of
geometry. With heads, the donor `]` sits deeper in the partner (bytes
10-60), not at byte 0, because armour works differently: the first bytes
now set the heads instead of being executed. All 7 lineages took over,
against ~55% in the default variant (small n).

**Prediction 1 met: direct copies, not mirrors.** In every replicator core
I dissected, both heads move the same way: `[>.}]` (both forward),
`[{.<]` and `{[{<,]` (both backward). In the default variant they move in
opposite directions (`[{.>]`, `[<,}]`), and that's exactly why every copy
there came out mirrored and only palindromes survived. With heads, the
first two bytes aim head1 into the partner, so nothing has to wrap.

**A blind spot.** Heads-variant replicators often copy only the part of
themselves that matters. Seed 506's copies ~69% (the first ~44 bytes),
and seeds 513 and 516 copy ~41-46% (just the core and a little padding).
So the lowered 60% threshold catches the first kind but not the second, and
the two seeds that took over through core-only copiers have no traced
birth. Lowering the threshold further would let degenerate byte-smearers
(programs that fill their partner with one repeated byte) count as copiers.
The right test would be "copies its *essential* bytes", which I haven't
built. On the default variant, 60% gives identical births to 90% (seeds 1
and 25), so the threshold change doesn't distort the comparison.

## Day 2, 15:00 — related work, and re-checking a claim

The owner found related work I hadn't seen: Knierim, Versari, Obryk,
Agüera y Arcas & Saurous, "BFF: Simple explanations for complex
phenomena" (arXiv:2607.01483, July 2026), from the original group. Using a
direct self-replication detector, they report that in their main variant
(`bff_selfmove`, a single copy instruction that also moves the head):
- uniform random 64-byte programs contain a replicator about 1 in 2.9x10^7;
- mutation walks with no interaction at all find replicators, often
  faster than the interacting soup;
- blocking "mergers" (multi-byte copies between programs) doesn't prevent
  emergence; it stops replicators taking over.

That bears directly on two of my claims. One is "noise essentially never
produces a replicator": I found 0 heritable in 10^9. But my test was much
stricter than theirs (one 4-generation chain at 90%, position-exact, and
that run predates my shifted-copy fix). The other is that sealing the seam
gave 0 births. The setups differ (language, soup, mutation, intervention),
so this isn't necessarily a contradiction, but I should measure rather than
argue.

Implemented their detector as described (`knierim_score`: 9 chains x 5
generations, P starts in the second half, score = min(first-half bytes
matching P in ≥3 chains, second-half bytes identical in ≥3 chains), ≥48 =
replicator; noise unspecified in the paper, uniform here). Unit test: the
hand-made and evolved replicators score ≥48, noise scores <20.

**Prediction before running:** in `bff_noheads` (my language, the original
paper's default), uniform random programs give **fewer than 5 replicators
per 10^9** by their detector, against ~34 per 10^9 for `bff_selfmove` from
their Table 1.

**Result: 0 replicators in 10⁹ uniform random programs** by their
detector (prediction: fewer than 5). 95% upper bound ~3 per 10⁹, so
`bff_noheads` has at least ~10x fewer replicators in noise than their
`bff_selfmove` (1 in 2.9x10^7). With the detector question settled, the
"noise alone doesn't do it" claim holds for this language. My reading of
the two papers together: where replicators are common (their language),
mutation finds them without help; where they're this rare, the soup gets
there mostly by partner completion, and blocking that stops it. Running the
birth forensics on `bff_selfmove` is the obvious next experiment.

Second run, same detector, bytes i.i.d. from the pre-life soup's
histogram (mixed seed 1, epoch 5,120, cleanly pre-life): **12 replicators
in 10⁹** (1.2x10^-8), against 0 for uniform bytes. They look like the soup's
own replicators: `[ < , } ]` loops with mirrored tails.

**This partly reverses my 03:00 correction.** I concluded then that
composition doesn't make replicators findable, from 2M samples with my
strict test (0 heritable). With 10⁹ samples and a fairer detector,
composition does help: at least ~4x over uniform. Knierim et al. report the
same direction for `bff_selfmove` (their soup-matched distribution is ~17x
faster than uniform). Replicators are still very rare (about 1 in 80
million), and the forensics and sealing results stand: in this soup they
mostly arrive by partner completion. But composition belongs in the story
as a second factor that sets the odds.
