"""Statistics of the selection rule (SPEC §G6): Wilson intervals, Bradley–Terry with ties,
seeded bootstrap intervals, order-conflict rate, rank correlation. Pure and deterministic
(seeded `random.Random`); unit-tested in tests/test_stats.py.
"""

from __future__ import annotations

import math
import random
from collections import defaultdict

Z95 = 1.959963984540054


def wilson(k: float, n: int, z: float = Z95) -> tuple[float, float, float]:
    """Proportion and Wilson score interval (k may be fractional: partial credit)."""
    if n == 0:
        return (float("nan"), 0.0, 1.0)
    p = k / n
    den = 1 + z * z / n
    c = (p + z * z / (2 * n)) / den
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / den
    return (p, max(0.0, c - h), min(1.0, c + h))


def pair_outcomes(judgements):
    """Combines the two orders of each (pair, replicate) (RESEARCH U1): the same winner in
    both orders = a win; a split, or 'cannot tell' in either = a tie.

    judgements: [{"a": design, "b": design, "rep": k, "order": 0|1, "winner": design|None}]
    (winner None = "ayırt edemiyorum"). Returns ([(x, y, result)], conflicts) where result
    is 1 (x wins), 0 (y wins) or 0.5 (tie), and conflicts = {(x, y): (split, total)}."""
    by = defaultdict(dict)
    for j in judgements:
        x, y = sorted((j["a"], j["b"]))
        by[(x, y, j["rep"])][j["order"]] = j["winner"]
    out, conf = [], defaultdict(lambda: [0, 0])
    for (x, y, _), w in sorted(by.items()):
        if len(w) < 2:
            continue
        w0, w1 = w[0], w[1]
        conf[(x, y)][1] += 1
        if w0 is not None and w0 == w1:
            out.append((x, y, 1.0 if w0 == x else 0.0))
        else:
            if w0 is not None and w1 is not None:
                conf[(x, y)][0] += 1
            out.append((x, y, 0.5))
    return out, {k: tuple(v) for k, v in conf.items()}


def bradley_terry(outcomes, items=None, iters: int = 2000, tol: float = 1e-10, prior: float = 0.5):
    """Bradley–Terry strengths (log scale, mean 0) by the MM algorithm (Hunter 2004); a tie
    counts half a win to each side. `prior`: a pseudo-tie of that weight between each
    design and a virtual average opponent, so that a design that never loses stays finite."""
    items = sorted(items or {d for o in outcomes for d in o[:2]})
    w = defaultdict(float)
    n = defaultdict(float)
    for x, y, r in outcomes:
        w[x] += r
        w[y] += 1 - r
        n[(x, y)] += 1
        n[(y, x)] += 1
    p = {i: 1.0 for i in items}
    for _ in range(iters):
        new = {}
        for i in items:
            den = sum(n[(i, j)] / (p[i] + p[j]) for j in items if j != i and n[(i, j)])
            num = w[i] + prior
            den += 2 * prior / (p[i] + 1.0)
            new[i] = num / den if den else p[i]
        g = math.exp(sum(math.log(v) for v in new.values()) / len(new))
        new = {i: v / g for i, v in new.items()}
        if max(abs(new[i] - p[i]) for i in items) < tol:
            p = new
            break
        p = new
    return {i: math.log(p[i]) for i in items}


def bootstrap_bt(outcomes, seed: int, reps: int = 2000, alpha: float = 0.05):
    """Percentile bootstrap of BT strengths, resampling outcomes with replacement (seeded).
    Returns {design: (estimate, lo, hi)} and the per-replicate rank of each design."""
    items = sorted({d for o in outcomes for d in o[:2]})
    est = bradley_terry(outcomes, items)
    rng = random.Random(seed)
    samples = defaultdict(list)
    for _ in range(reps):
        s = [outcomes[rng.randrange(len(outcomes))] for _ in outcomes]
        bt = bradley_terry(s, items, iters=500)
        for i in items:
            samples[i].append(bt[i])
    out = {}
    for i in items:
        v = sorted(samples[i])
        lo = v[int(math.floor(alpha / 2 * reps))]
        hi = v[min(reps - 1, int(math.ceil((1 - alpha / 2) * reps)) - 1)]
        out[i] = (est[i], lo, hi)
    return out


def bootstrap_mean(values, seed: int, reps: int = 2000, alpha: float = 0.05):
    """Mean and percentile bootstrap interval of a list of numbers (seeded)."""
    if not values:
        return (float("nan"), float("nan"), float("nan"))
    rng = random.Random(seed)
    m = sum(values) / len(values)
    bs = sorted(sum(values[rng.randrange(len(values))] for _ in values) / len(values) for _ in range(reps))
    return (m, bs[int(alpha / 2 * reps)], bs[min(reps - 1, int((1 - alpha / 2) * reps))])


def overlaps(a, b) -> bool:
    """Do two (estimate, lo, hi) intervals overlap?"""
    return not (a[1] > b[2] or b[1] > a[2])


def spearman(xs, ys) -> float:
    """Spearman rank correlation (average ranks for ties)."""
    def ranks(v):
        order = sorted(range(len(v)), key=lambda i: v[i])
        r = [0.0] * len(v)
        i = 0
        while i < len(v):
            j = i
            while j + 1 < len(v) and v[order[j + 1]] == v[order[i]]:
                j += 1
            for k in range(i, j + 1):
                r[order[k]] = (i + j) / 2 + 1
            i = j + 1
        return r
    if len(xs) < 3:
        return float("nan")
    rx, ry = ranks(xs), ranks(ys)
    mx, my = sum(rx) / len(rx), sum(ry) / len(ry)
    num = sum((a - mx) * (b - my) for a, b in zip(rx, ry))
    den = math.sqrt(sum((a - mx) ** 2 for a in rx) * sum((b - my) ** 2 for b in ry))
    return num / den if den else float("nan")
