# A borrowed bracket

*How self-replicating programs come into being in a soup of random code.*

Take 32,400 strings of 64 random bytes and lay them out on a grid. Over and
over, pick two neighbours, glue them end to end into a 128-byte tape, and
run the tape as a program. Then cut it back in half and put the halves
back. Nothing is rewarded or selected, and nothing is told to copy itself.

The language is BFF, a Brainfuck dialect from Agüera y Arcas et al.,
["Computational Life"](https://arxiv.org/abs/2406.19108) (2024). Ten of the
256 byte values are instructions: move one of two read/write heads, add or
subtract one, copy a byte from one head to the other, and loop with `[` and
`]`. Every other byte does nothing. Code and data share the tape, so a
program can rewrite its own instructions and its neighbour's.

For thousands of rounds nothing much happens. Then, at some unpredictable
moment, one tile on the grid becomes a program that writes a copy of itself
into whatever it touches. Its copies do the same. A front spreads across
the grid, and within a thousand rounds the noise is gone.

![The 2D soup coming alive](media/takeover_frames.png)

*Seed 1, epochs 8,000 to 12,200. Each tile is one program and each colour
an instruction. The grey region is replicators.*

The paper reproduces well: my implementation came alive at a similar rate
(2 of 8 well-mixed runs within 16k epochs, against their ~40%). This essay is about the question the paper leaves open: **what
exactly happens at the moment the first replicator appears?**

## Replicators are mirror twins

The replicators that win are small loops such as `[ < , } ]`: move head0
back one byte, copy the byte under head1 to head0, move head1 forward, and
repeat. Both heads start at byte 0. Stepping head0 backwards from byte 0
wraps around to byte 127, the far end of the partner. So the loop reads the
program forwards and writes it into the partner backwards. **Every copy is
a mirror image.**

A mirrored loop is usually not a loop: `[ < , } ]` reversed reads
`] } , < [`. The replicators that survive are near-palindromes, with a
reversed second copy of the loop in their tail, so that the mirror image
works too. In the well-mixed soup this shows up as twins. The commonest
program at takeover had 263 copies and its mirror image had 261; the next
pair, 201 and 198. Each lineage flips orientation every generation.

## Catching a birth

The simulation is deterministic, so a run can be replayed exactly. That
makes forensics possible. Replay the run, test every program each epoch for
whether it copies itself, find the first epoch with a copier, and then find
the single interaction that produced it.

The first one I caught, at epoch 7,904:

```
parent   [·[·[·[·[·[···,·························<···,·········}·}·······
partner  ]························{···························[····00000·
            ↓  one interaction, 1,740 instructions
partner  [[[·······<,··}·]······························]·}··,<·······[[[
```

The parent is not a replicator. It holds the pieces of a copy loop
(`[ , < , } }`) but nothing to close it with. Its partner happens to begin
with `]`. Glued together they form one tape, and the parent's loop closes
on the partner's bracket, across the seam between the two programs. It
runs 63 times and writes a cleaned-up, mirrored copy of the parent's code
over the partner. That copy is a complete replicator, with its own `]`.

The first replicator was the joint product of two programs, neither of
which could copy itself.

## It's not a fluke

One case is an anecdote, so I automated it. I ran 60 independent 2D soups,
traced every new copying lineage back to the interaction that created it,
and recorded whether the creating loop crossed the seam.

| how the first copier came to be | births |
|---|---|
| the parent's loop closes on a `]` in the partner (seam loop) | **42** |
| the parent runs only if the partner has a zero at byte 126 ("lock and key") | 1 |
| a byte copied in from the partner completes the parent | 2 |
| a mutation deletes a premature `]`, lengthening a loop | 2 |
| ambiguous | 2 |

**45 of 49 births were a latent copier completed by its partner.** About
half of the new lineages died within a few hundred epochs; the rest took
over.

## Why brackets are lying around

In a soup with no life yet, 10-14% of programs start with `]`. That is 29
times more often than `]` appears at any other position. The reason has
nothing to do with replication. A program whose first byte is `]` halts
instantly when it runs: `]` looks backwards for a matching `[`, finds the
start of the tape, and stops. That protects it from rewriting itself into
garbage, so these "armoured" programs persist.

Armour is exactly what the incomplete copiers need. A first-half program's
loop that runs off its own end lands on byte 64, the partner's first byte.
In 17 of the 42 seam births, that is where the borrowed `]` was.

## Testing it by changing the rules

Watching is not proof, so I changed the physics and predicted the result
in writing before each run.

**Seal the seam.** Change the interpreter so a `[` in one program can't
match a `]` in the other. Replicators that carry their own brackets are
unaffected. Result: **zero births in 3 million epochs**, against about 50
expected. One complication: sealing also leaves the pre-life soup poorer in
instructions, because cross-seam loops help spread them around. So I also
sealed soups that had already matured under normal rules, for short
windows. Those gave zero births, against 9 from the same starting soups
left unsealed, four of which arrived within the first 400 epochs.

**Strip the armour.** Each epoch, replace any leading `]` with another
byte. Births fell **threefold**. I had predicted a smaller drop, which
suggests armour matters for more than the byte-0 bracket itself.

**Change the geometry.** In the paper's other BFF variant, the first two
bytes aim the two heads. A program can then point a head straight into its
partner, and replicators become direct, unmirrored copiers. If the borrowed
bracket were an accident of the mirror trick, it should disappear here. It
didn't: all 7 births I traced in that variant were completed by the
partner, 5 of them through a seam loop.

## What it adds up to

Pure noise essentially never produces a replicator in this language. Of a
billion random programs, several thousand copied themselves once, and none
produced a copy that could copy again. A second billion, scored with the
independent detector described below, also produced none. Drawing bytes
with the pre-life soup's frequencies instead gives about 1 replicator per
80 million programs. The soup's chemistry helps, but not enough on its
own. What the soup produces instead is a large
supply of almost-copiers: loops missing their bracket, gated on a byte,
one instruction short. A replicator appears when one of them meets a
neighbour that happens to carry the missing piece, and the copy it writes
has that piece built in.

## Related work

After finishing, I learned of Knierim, Versari, Obryk, Agüera y Arcas and
Saurous, ["BFF: Simple explanations for complex
phenomena"](https://arxiv.org/abs/2607.01483) (2026), by the original
group. They argue that interaction isn't an especially powerful way to find
replicators. In their main variant, `bff_selfmove`, a single instruction
both copies a byte and moves the head. There, about 1 in 2.9×10⁷ uniform
random programs is already a replicator, and plain mutation walks with no
partners find replicators readily. Blocking copies between programs slows
takeover but doesn't prevent emergence.

That sounds like the opposite of my story, so I implemented their detector
as the paper describes it and ran it over a billion random programs in my
language, `bff_noheads`. It found none, which puts replicators there at
least about ten times rarer than in theirs. In `bff_noheads` a copy loop
needs around five particular instructions in the right order, plus a
mirrored tail, while their language does the copy and the step in one
byte. My reading is that both results hold. Where replicators are common,
mutation finds them on its own. Where they are as rare as here, the soup
gets there mostly by having a partner complete an almost-copier, and
blocking that stops it. The obvious next experiment is to run the
borrowed-bracket forensics on `bff_selfmove`.

## What I got wrong along the way

- An early "64x" enrichment figure came from a soup sample that already
  contained thousands of replicators. With clean samples the effect mostly
  disappeared, and I retracted it.
- My first copier test compared bytes position by position. Many copies
  come out shifted by a byte or two, so the test missed real lineages and
  reported false "births". I had to widen it twice.
- I first concluded that the pre-life soup's byte composition doesn't make
  replicators easier to find. A billion samples scored with a fairer
  detector showed it does help, at least fourfold. I had over-corrected
  from too small a sample.
- I briefly believed one batch of seeds came alive faster than another (p ≈
  0.002), but I had chosen the split after looking at the data. A fresh
  test with the prediction written down first came out exactly as a
  uniform rate predicts.

Code, data, the journal (with every mistake left in place) and a browser
version with replays of eleven real births are in this repository; see
README.md.
