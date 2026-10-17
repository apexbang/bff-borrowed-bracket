"""Summarise births/*.txt output into one table plus tallies.

usage: python3 births_summary.py runs/births_v2/s14.txt runs/births/s23.txt ...
"""
import re
import sys

rows = []
for path in sys.argv[1:]:
    text = open(path).read()
    seed = re.search(r"seed (\d+)", text).group(1)
    blocks = re.split(r"(?=seed \d+ \| heritable)", text)
    end = re.search(r"seed \d+ \| (TRANSITION at epoch (\d+)|no transition)", text)
    for b in blocks:
        if "heritable" not in b:
            continue
        m = re.search(r"BIRTH epoch (\d+)", b)
        if not m:
            rows.append((seed, "?", "untraced", "", "?"))
            continue
        epoch = m.group(1)
        if "mutation" in b:
            ch = re.search(r"changed (.*?)  \|", b)
            mech, detail = "mutation", ch.group(1) if ch else ""
        else:
            seams = re.search(r"seam loops: (.*)", b).group(1)
            if seams.strip() == "none":
                # How many visible bytes of the tile changed (instructions, zeros)?
                half = re.search(r"while (first|second) half", b).group(1)
                before = re.search(rf"    {half}\s+(.*)", b).group(1)
                after = re.search(r"    after\s+(.*)", b).group(1)
                after = after[:64] if half == "first" else after[64:]
                n = sum(x != y for x, y in zip(before, after))
                mech, detail = "no seam loop", f"{n} visible bytes of the tile changed"
            else:
                first = re.match(r"\]@(\d+)->\[@(\d+)", seams)
                f, t = int(first.group(1)), int(first.group(2))
                mech = "seam loop"
                detail = f"] at partner byte {f - 64}" if f >= 64 else f"] at byte {f}, [ in partner"
        fate = "died" if "lineage gone" in b else ("took over" if end and end.group(2) else "?")
        rows.append((seed, epoch, mech, detail, fate))

print("| seed | birth epoch | mechanism | detail | fate |")
print("|---|---|---|---|---|")
for r in rows:
    print("| " + " | ".join(r) + " |")
mechs = {}
for r in rows:
    mechs.setdefault(r[2], [0, 0])
    mechs[r[2]][0] += 1
    mechs[r[2]][1] += r[4] == "took over"
print()
for k, (n, ok) in mechs.items():
    print(f"{k}: {n} births, {ok} took over")
