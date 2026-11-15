"""Denetçinin kendi sentetik bar üreteci (qp-source kullanılmaz).

Rejim parçaları: yükseliş, düşüş, dar bant (sıkışma, sonunda sert çıkış), oynak, testere;
ayrıca hacim sıçramaları ve ayrı bir "kurumsal net" serisi (bazı barlarda nokta yok / net yok).
Kullanım: python3 -I gen_bars.py <çıktı.json> <tohum> <enstrüman> <bar>
"""
import json
import math
import sys


class Lcg:
    def __init__(self, s):
        self.s = (s * 2862933555777941757 + 3037000493) % (1 << 64)

    def u(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % (1 << 64)
        return (self.s >> 11) / float(1 << 53)

    def n(self):
        a = max(self.u(), 1e-12)
        b = self.u()
        return math.sqrt(-2 * math.log(a)) * math.cos(2 * math.pi * b)


def instrument(seed, n):
    r = Lcg(seed)
    c = 50 + 100 * r.u()
    bars, flow = [], []
    t0 = 1_600_000_000_000
    regime, left = 0, 0
    for i in range(n):
        if left <= 0:
            regime = int(r.u() * 5)
            left = 15 + int(r.u() * 60)
        left -= 1
        drift, vol = {0: (0.004, 0.012), 1: (-0.004, 0.012), 2: (0.0, 0.002),
                      3: (0.0, 0.03), 4: (0.0, 0.01)}[regime]
        if regime == 2 and left == 1:  # dar banttan sert çıkış
            drift = 0.06 if r.u() < 0.5 else -0.06
        o = c * (1 + 0.002 * r.n())
        c = max(0.5, o * (1 + drift + vol * r.n()))
        h = max(o, c) * (1 + vol * 0.5 * r.u())
        l = min(o, c) * (1 - vol * 0.5 * r.u())
        v = 1e5 * (1 + 3 * r.u())
        if r.u() < 0.03:
            v *= 3 + 4 * r.u()  # hacim sıçraması
        bars.append([t0 + 86_400_000 * i, o, h, l, c, v])
        x = r.u()
        if x < 0.04:
            flow.append(None)  # nokta yok
        elif x < 0.06:
            flow.append("nonet")  # net yok
        else:
            net = v * 0.05 * r.n()
            if r.u() < 0.05:
                net *= 6
            flow.append(round(net, 2))
    return bars, flow


def main():
    out, seed, k, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
    data = []
    for j in range(k):
        bars, flow = instrument(seed * 1000 + j, n)
        data.append({"symbol": f"AUD{j}", "bars": bars, "flow": flow})
    with open(out, "w") as f:
        json.dump(data, f)


main()
