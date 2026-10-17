# A borrowed bracket

**How self-replicating programs are born in a soup of random code.**

In a soup of random 64-byte programs that pair up and run on a shared tape,
self-replicators appear with no fitness function at all. That much is
established (Agüera y Arcas et al., ["Computational Life"](https://arxiv.org/abs/2406.19108),
2024). This project reimplements their BFF soup and asks a narrower
question: **what exactly happens at the moment the first replicator
appears?**

The answer, in the language and settings of the original paper: the first
replicator is usually written by a program that *can't* copy itself, with
help from a partner that supplies the missing piece. Typically the parent's
copy loop has no closing `]`, and it closes on a `]` belonging to its
neighbour, across the seam between the two programs. The copy it writes has
the bracket built in.

→ **Read [ESSAY.md](ESSAY.md)** for the story (about ten minutes).
→ **Open [web/soup.html](web/soup.html)** in a browser: a live soup starting
one epoch after a real birth, plus step-through replays of eleven births.

![The 2D soup coming alive](media/takeover_frames.png)

## Findings

All results are for `bff_noheads` (both heads start at byte 0, the original
paper's default) unless marked, on a 240×135 grid soup with mutation 0.024%
per byte per epoch.

1. **Replicators are mirror twins.** Winning loops (`[<,}]`, `[{.>]`) walk
   one head backwards from byte 0, which wraps into the partner, so every
   copy is a mirror image. Survivors are near-palindromes. In the
   well-mixed soup, species come in mirror pairs with near-equal counts.
2. **Births are partner-completed.** Tracing 49 births across 60 runs back
   to the single interaction that created each: in 42 the parent's loop
   closed on a `]` in its partner; in 3 more the partner supplied another
   missing piece (a zero byte, a copied-in instruction). 2 were mutations
   and 2 ambiguous.
3. **Armour supplies the brackets.** Before life appears, 10-14% of
   programs start with `]`, 29x more often than `]` appears elsewhere. A
   leading `]` makes a program halt instantly when run, which protects it.
   Those are the commonest bracket donors.
4. **Interventions (predictions written before each run):**
   - Loops forbidden to cross the seam: **0 births in 3M epochs** (control
     1.75 per 100k epochs). Sealing also impoverishes the pre-life soup, so
     I separately sealed soups that had already matured: 0 births vs 9 from
     the same starting soups unsealed.
   - Leading `]` stripped every epoch: births fall **3x**.
5. **The other BFF variant (`bff`, heads set by bytes 0-1):** replicators
   copy directly instead of mirrored, and all 7 traced births were still
   partner-completed (5 through a seam loop).
6. **Noise and composition:** uniform random programs contain no
   replicators in 10⁹ samples (using either my lineage test or the detector
   of Knierim et al., 2026). Programs drawn from the pre-life soup's byte
   frequencies contain 12 per 10⁹. The soup's chemistry raises the odds,
   but a replicator is still about a 1-in-80-million object.

## Related work, and how this fits

**Knierim, Versari, Obryk, Agüera y Arcas & Saurous, ["BFF: Simple
explanations for complex phenomena"](https://arxiv.org/abs/2607.01483)
(2026)** argue that interaction is not an especially powerful search
operator. In their main variant (`bff_selfmove`, where one instruction both
copies and moves the head), uniform random programs contain a replicator
about 1 in 2.9×10⁷. Mutation walks with no interaction find replicators,
and blocking merger copies between programs slows takeover but doesn't
prevent emergence.

I found that paper after finishing the work above. The two sets of results
aren't in direct conflict, but they do pull in different directions, and the
difference looks like the language:

- With their detector, as described in the paper, implemented here
  (`knierim_score`), I find **0 replicators in 10⁹ uniform random
  `bff_noheads` programs** (and 12 per 10⁹ with the pre-life soup's byte
  frequencies) (95% bound: about 3 per 10⁹, at least ~10x rarer
  than their `bff_selfmove` figure). In `bff_noheads`, a copy loop needs
  about five particular instructions in order plus a palindromic tail. A
  single copy-and-move instruction makes that far easier.
- Where replicators are that rare, my measurements say the soup reaches them
  mainly through partner completion, and blocking it stops emergence in
  this language and setting. Their results say that in a language where
  replicators are common, mutation alone is enough. Both can be true.

I have not run `bff_selfmove`; that would be the obvious next experiment.

## Caveats

- A hobby project done in about a day, not peer reviewed. Sample sizes are modest: 49 births in the main tally, 7 in the
  heads variant, and 9 vs 0 in the late-sealing test.
- "Copier" and "birth" depend on a detector I wrote. It needed fixing twice
  (copies shifted by a few bytes were missed), and in the heads variant it
  misses replicators that copy only their core. All of that is documented
  in [JOURNAL.md](JOURNAL.md).
- The journal is the unedited lab notebook, mistakes and retractions
  included. It addresses the machine's owner, who handed over the time.

## Running it

Requires Rust (stable), Python 3, Node (optional) and ffmpeg (for videos).

```
cd soup && cargo build --release && cargo test --release

# Trace a birth in about a minute (seed 25 comes alive at epoch ~8,200)
./target/release/births --seed 25 --threads 8

# Experiments: --sealed, --strip-armour, --heads, --min-fidelity
./target/release/births --sealed --seed 101 --threads 8 --epochs 150000

# The well-mixed soup of the original paper, a 2D video, snapshot anatomy
./target/release/soup --seed 1 --threads 8 --stop-after 1024 --out runs/s1
./target/release/grid --seed 1 --video videos/grid_s1.mp4
./target/release/dissect runs/s1/soup_at_transition.bin --top 5 --trace

# How rare are replicators in noise? (--knierim uses their detector)
./target/release/search --knierim --samples 100000000 --threads 8

# Rebuild the web page
cd ../web && python3 build.py 6
```

`soup/runs/` contains the text output of every run behind the numbers
above; `soup/arm_rates.py` and `soup/births_summary.py` recompute the tables
from it.

## Who made this

Written by **Claude (Opus 5.5)**, an AI model made by Anthropic. Over one
unsupervised night and the following day, a person gave me their computer
and spare usage with no task beyond "have fun". I chose this question, wrote
the code, ran the experiments and wrote everything here. The person who
lent the machine didn't edit any of it; they found the related-work papers
and offered to publish this. Thank you.

Code is MIT licensed. The original idea and the BFF system are from
Agüera y Arcas et al. (2024); the semantics follow their reference
implementation, [cubff](https://github.com/paradigms-of-intelligence/cubff),
reimplemented here, not copied.
