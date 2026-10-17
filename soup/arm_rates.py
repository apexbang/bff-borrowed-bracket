"""Birth and takeover rates per experimental arm, with exact 95% Poisson CIs.

usage: python3 arm_rates.py name=glob [name=glob ...]
e.g.   python3 arm_rates.py sealed='runs/sealed/*.txt' noarmour='runs/noarmour/*.txt'
The control arm (last night's runs) is always included.
"""
import glob
import math
import re
import sys

CONTROL = (["runs/births_v2/s1.txt"]
           + [f"runs/births_v2/s{s}.txt" for s in range(14, 23)]
           + [f"runs/births_v2/s{s}.txt" if s in (34, 36) else f"runs/births/s{s}.txt" for s in range(23, 41)]
           + [f"runs/births_fresh/s{s}.txt" for s in range(41, 61)])


def poisson_cdf(k, mu):
    term = total = math.exp(-mu)
    for i in range(1, k + 1):
        term *= mu / i
        total += term
    return total


def poisson_ci(k, conf=0.95):
    a = (1 - conf) / 2

    def solve(f, lo, hi):
        for _ in range(200):
            mid = (lo + hi) / 2
            lo, hi = (mid, hi) if f(mid) else (lo, mid)
        return (lo + hi) / 2

    lower = 0.0 if k == 0 else solve(lambda m: 1 - poisson_cdf(k - 1, m) < a, 0, 10 * k + 10)
    upper = solve(lambda m: poisson_cdf(k, m) > a, 0, 10 * k + 10)
    return lower, upper


def arm(files):
    exposure = births = seam = takeovers = 0
    for f in files:
        text = open(f).read()
        m = re.search(r"TRANSITION at epoch (\d+)", text)
        done = re.search(r"no transition in (\d+)", text)
        if not (m or done):
            continue  # still running
        exposure += int(m.group(1)) if m else int(done.group(1))
        births += text.count("BIRTH")
        seam += len(re.findall(r"seam loops: \]", text))
        takeovers += bool(m)
    return len(files), exposure, births, seam, takeovers


arms = [("control", CONTROL)] + [(a.split("=", 1)[0], sorted(glob.glob(a.split("=", 1)[1]))) for a in sys.argv[1:]]
print("| arm | runs | epochs | births | via seam | births /100k (95% CI) | takeovers /100k (95% CI) |")
print("|---|---|---|---|---|---|---|")
for name, files in arms:
    n, e, b, s, t = arm(files)
    if e == 0:
        continue
    k = 1e5 / e
    bl, bh = poisson_ci(b)
    tl, th = poisson_ci(t)
    print(f"| {name} | {n} | {e:,} | {b} | {s} | {b*k:.2f} ({bl*k:.2f}-{bh*k:.2f}) | {t*k:.2f} ({tl*k:.2f}-{th*k:.2f}) |")
