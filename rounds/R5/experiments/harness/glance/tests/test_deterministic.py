"""Unit tests of the deterministic parts (SPEC §G10): keys, answer parsing, colour maths,
statistics, selection rule, broken-design verdict. Run: python3 -m unittest discover -s tests
"""

import os
import random
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from gl import color as C  # noqa: E402
from gl import compute as CP  # noqa: E402
from gl import keys as K  # noqa: E402
from gl import readers as R  # noqa: E402
from gl import select as SEL  # noqa: E402
from gl import stats as S  # noqa: E402

DAY = 86_400_000


def item(**kw):
    base = {"kind": "breakout", "direction": "up", "validity": "valid", "stage": "born", "time": "event",
            "t": 1_700_000_000_000, "ledger_at_birth": {"n": 50, "realized": 35}, "trust": "replica"}
    base.update(kw)
    return base


class Keys(unittest.TestCase):
    def test_face_keys(self):
        it = item()
        k = K.keys_for("face", {"item": it, "now_t": it["t"]})
        self.assertEqual(k["dir"], {"up"})
        self.assertEqual(k["kind"], {"breakout"})
        self.assertEqual(k["valid"], {"valid"})
        self.assertEqual(k["when"], {"today"})
        self.assertEqual(k["trust"], {"mostly"})
        self.assertEqual(k["advice"], {"nothing"})

    def test_trust_buckets(self):
        self.assertEqual(K.trust_key(item(ledger_at_birth={"n": 19, "realized": 19}), False), "few")
        self.assertEqual(K.trust_key(item(ledger_at_birth={"n": 0, "realized": 0}), False), "few")
        self.assertEqual(K.trust_key(item(ledger_at_birth={"n": 20, "realized": 10}), False), "half")
        self.assertEqual(K.trust_key(item(ledger_at_birth={"n": 20, "realized": 7}), False), "rarely")
        self.assertEqual(K.trust_key(item(), True), "hidden")

    def test_valid_and_dir(self):
        self.assertEqual(K.valid_key(item(validity="closed", stage="realized")), "realized")
        self.assertEqual(K.valid_key(item(validity="closed", stage="invalidated")), "failed")
        self.assertEqual(K.valid_key(item(validity="closed", stage="expired")), "expired")
        self.assertEqual(K.valid_key(item(validity="no_longer", stage="weakened")), "no_longer")
        self.assertEqual(K.dir_key(item(direction="neutral")), "flat")

    def test_when_friday_monday(self):
        # 2023-06-16 is a Friday; read on Monday 2023-06-19 (Istanbul).
        fri = 1686898800000
        mon = fri + 3 * DAY
        self.assertEqual(K.when_keys(fri, mon), {"days", "yesterday"})
        self.assertEqual(K.when_keys(fri, fri + 10 * DAY), {"week"})

    def test_visitor_and_feed(self):
        it = item(redacted=True)
        k = K.keys_for("face_visitor", {"item": it, "now_t": it["t"], "redacted": True})
        self.assertEqual(k["numbers"], {"no_license"})
        self.assertEqual(k["trust"], {"hidden"})
        vis = [item(direction="down"), item(validity="closed", stage="realized"), item()]
        k = K.keys_for("feed", {"visible": vis})
        self.assertEqual(k["f_live"], {"2"})
        self.assertEqual(k["f_down"], {"yes"})

    def test_credit_and_chance(self):
        self.assertEqual(K.credit("dir", "up", {"up"}), 1.0)
        self.assertEqual(K.credit("dir", K.UNKNOWN, {"up"}), 0.0)
        self.assertEqual(K.credit("dir", None, {"up"}), 0.0)
        self.assertEqual(K.credit("t_first", "sentence", set()), 0.5)
        self.assertAlmostEqual(K.chance("dir"), 1 / 3)
        self.assertAlmostEqual(K.chance("t_first"), 1.5 / 6)


class Parsing(unittest.TestCase):
    def test_prompt_is_blind_and_deterministic(self):
        p1, l1 = R.build_prompt("face", "tr", "F", "x1y2.png", 7)
        p2, l2 = R.build_prompt("face", "tr", "F", "x1y2.png", 7)
        self.assertEqual((p1, l1), (p2, l2))
        self.assertEqual(R.check_prompt(p1, ["vadrel", "fotolay", "kestim"]), [])
        self.assertIn("vadrel", R.check_prompt("Tasarım vadrel", ["vadrel"]))
        # don't-know is always the last letter
        for spec in l1.values():
            self.assertEqual(list(spec["map"].values())[-1], K.UNKNOWN)

    def test_one_sentence_cut(self):
        kept, cut = R.one_sentence("B, çünkü ok yukarı. Ayrıca başka bir şey var.")
        self.assertEqual(kept, "B, çünkü ok yukarı.")
        self.assertTrue(cut)
        kept, cut = R.one_sentence("A")
        self.assertEqual((kept, cut), ("A", False))
        kept, cut = R.one_sentence("C " + "x" * 400)
        self.assertEqual(len(kept), R.MAX_SENTENCE)
        self.assertTrue(cut)

    def test_parse_answer(self):
        _, letters = R.build_prompt("gray", "en", "H", "f.png", 3)
        m1 = letters["q1"]["map"]
        a_code = m1["A"]
        res = '```json\n{"q1": "A, the arrow points that way. More.", "q2": "nonsense"}\n```'
        ans = R.parse_answer(res, letters)
        self.assertEqual(ans["dir"]["code"], a_code)
        self.assertTrue(ans["dir"]["cut"])
        self.assertIsNone(ans["valid"]["code"])
        self.assertFalse(ans["valid"]["missing"])

    def test_parse_critic(self):
        self.assertEqual(R.parse_critic('{"gozlem": "x.", "karar": "B", "neden": "y."}')["verdict"], "B")
        self.assertIsNone(R.parse_critic('{"gozlem": "x.", "karar": "ayırt edemiyorum", "neden": "y."}')["verdict"])
        self.assertEqual(R.parse_critic('{"observation": "x", "verdict": "A", "why": "y"}')["verdict"], "A")


class Colour(unittest.TestCase):
    def test_contrast(self):
        self.assertAlmostEqual(C.contrast((0, 0, 0), (255, 255, 255)), 21.0, places=6)
        self.assertAlmostEqual(C.contrast((119, 119, 119), (255, 255, 255)), 4.48, places=2)

    def test_de2000_sharma(self):
        # Sharma, Wu & Dalal (2005), test data pairs 1, 7, 17.
        self.assertAlmostEqual(C.de2000((50, 2.6772, -79.7751), (50, 0, -82.7485)), 2.0425, places=4)
        self.assertAlmostEqual(C.de2000((50, 0, 0), (50, -1, 2)), 2.3669, places=4)
        self.assertAlmostEqual(C.de2000((50, 2.5, 0), (73, 25, -18)), 27.1492, places=4)

    def test_cvd_fixed_points(self):
        for k in ("protan", "deutan", "tritan"):
            for m in ("vienot_brettel", "machado"):
                w = C.simulate((255, 255, 255), k, m)
                self.assertTrue(all(abs(v - 255) < 1.0 for v in w), (k, m, w))
                g = C.simulate((128, 128, 128), k, m)
                self.assertTrue(all(abs(v - 128) < 1.5 for v in g), (k, m, g))

    def test_red_green_collapse_for_deutan(self):
        red, green = (200, 40, 40), (40, 160, 40)
        self.assertGreater(C.de(red, green), 40)
        self.assertLess(C.pair_de(red, green, "deutan"), C.de(red, green) / 2)

    def test_composite(self):
        self.assertEqual(C.composite((0, 0, 0, 0), (10, 20, 30)), (10, 20, 30))
        self.assertEqual(C.composite((0, 0, 0, 255), (10, 20, 30)), (0, 0, 0))


class Compute(unittest.TestCase):
    def scene(self, glyph_alpha=255, text=(30, 30, 30, 255)):
        return {"background": [255, 255, 255, 255], "under": [], "over": [], "panels": [{"layers": [
            {"id": "glyph", "items": [{"hit": {"left": 8, "top": 34, "right": 40, "bottom": 66}}],
             "prims": [{"prim": "fill", "color": [200, 0, 0, glyph_alpha], "points": [[8, 34], [40, 34], [40, 66], [8, 66]]}]},
            {"id": "headline", "items": [], "prims": [
                {"prim": "fill", "color": [0, 0, 0, 255], "points": [[50, 30], [300, 30], [300, 60], [50, 60]]},
                {"prim": "text", "x": 60, "baseline": 50, "px": 16, "color": list(text), "text": "Ab"}]}]}]}

    def test_text_over_dark_fill(self):
        t, g = CP.face_elements(self.scene())
        self.assertEqual(t[0]["bg"], (0.0, 0.0, 0.0))
        self.assertLess(t[0]["ratio"], 2.0)  # dark text on the dark fill under it
        self.assertEqual(g["size"], 32)

    def test_faint_glyph_fails(self):
        _, g = CP.face_elements(self.scene(glyph_alpha=46))
        self.assertLess(g["ratio"], 3.0)
        _, g = CP.face_elements(self.scene(glyph_alpha=255))
        self.assertGreater(g["ratio"], 3.0)

    def test_colour_table(self):
        sem = {"open/valid:up": {"fg": (20, 140, 60), "bg": (255, 255, 255), "size": 32},
               "open/valid:down": {"fg": (200, 40, 40), "bg": (255, 255, 255), "size": 32}}
        rows = CP.colour_table(sem)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["threshold"], CP.DE_MIN)
        self.assertFalse(rows[0]["pass"])  # red/green collapse under deutan/protan


class Stats(unittest.TestCase):
    def test_wilson(self):
        p, lo, hi = S.wilson(8, 10)
        self.assertAlmostEqual(p, 0.8)
        self.assertAlmostEqual(lo, 0.4902, places=3)
        self.assertAlmostEqual(hi, 0.9433, places=3)

    def test_pair_outcomes_orders(self):
        j = [{"a": "x", "b": "y", "rep": 0, "order": 0, "winner": "x"},
             {"a": "x", "b": "y", "rep": 0, "order": 1, "winner": "x"},
             {"a": "x", "b": "y", "rep": 1, "order": 0, "winner": "x"},
             {"a": "x", "b": "y", "rep": 1, "order": 1, "winner": "y"},
             {"a": "x", "b": "y", "rep": 2, "order": 0, "winner": None},
             {"a": "x", "b": "y", "rep": 2, "order": 1, "winner": "y"}]
        out, conf = S.pair_outcomes(j)
        self.assertEqual([o[2] for o in out], [1.0, 0.5, 0.5])
        self.assertEqual(conf[("x", "y")], (1, 3))

    def test_bt_order_and_bootstrap_determinism(self):
        outs = [("a", "b", 1.0)] * 8 + [("a", "b", 0.0)] * 2 + [("b", "c", 1.0)] * 8 + [("b", "c", 0.0)] * 2 \
            + [("a", "c", 1.0)] * 9 + [("a", "c", 0.0)]
        bt = S.bradley_terry(outs)
        self.assertGreater(bt["a"], bt["b"])
        self.assertGreater(bt["b"], bt["c"])
        b1 = S.bootstrap_bt(outs, seed=1, reps=200)
        b2 = S.bootstrap_bt(outs, seed=1, reps=200)
        self.assertEqual(b1, b2)
        for d, (e, lo, hi) in b1.items():
            self.assertLessEqual(lo, hi)

    def test_bt_ties_equal(self):
        bt = S.bradley_terry([("a", "b", 0.5)] * 10)
        self.assertAlmostEqual(bt["a"], bt["b"], places=6)

    def test_spearman(self):
        self.assertAlmostEqual(S.spearman([1, 2, 3, 4], [10, 20, 30, 40]), 1.0)
        self.assertAlmostEqual(S.spearman([1, 2, 3, 4], [4, 3, 2, 1]), -1.0)


def synth_rows(acc_by_design, n=40, seed=0):
    rng = random.Random(seed)
    rows = []
    for d, acc in acc_by_design.items():
        for i in range(n):
            for persona in ("F", "H"):
                for q in ("dir", "kind", "valid"):
                    rows.append({"design": d, "qset": "face", "stim": f"{d}{i}", "persona": persona, "rep": 0,
                                 "q": q, "credit": 1.0 if rng.random() < acc else 0.0, "cut": False, "one_look": True})
                rows.append({"design": d, "qset": "throttle", "stim": f"{d}{i}t", "persona": persona, "rep": 0,
                             "q": "t_first", "credit": 1.0 if rng.random() < acc else 0.0, "cut": False, "one_look": True})
    return rows


class Select(unittest.TestCase):
    def test_clear_winner_and_broken_last(self):
        rows = synth_rows({"p": 0.9, "q": 0.6, "z": 0.2})
        judg = []
        for rep in range(6):
            for o in (0, 1):
                judg += [{"a": "p", "b": "q", "rep": rep, "order": o, "winner": "p"},
                         {"a": "p", "b": "z", "rep": rep, "order": o, "winner": "p"},
                         {"a": "q", "b": "z", "rep": rep, "order": o, "winner": "q"}]
        res = SEL.select(rows, judg, {}, seed=5, broken="z")
        self.assertEqual(res["verdict"]["winner"], "p")
        self.assertTrue(res["verdict"]["told_apart"])
        v = SEL.broken_verdict(res, "z")
        self.assertTrue(v["valid"])

    def test_not_told_apart(self):
        rows = synth_rows({"p": 0.70, "q": 0.70}, n=15, seed=3)
        res = SEL.select(rows, [], {}, seed=5)
        self.assertFalse(res["verdict"]["told_apart"])
        self.assertIsNone(res["verdict"]["winner"])
        self.assertEqual(set(res["verdict"]["not_told_apart"]), {"p", "q"})
        self.assertIn(res["verdict"]["default"], {"p", "q"})

    def test_broken_not_last_invalidates(self):
        rows = synth_rows({"p": 0.5, "z": 0.9})
        res = SEL.select(rows, [], {}, seed=5, broken="z")
        self.assertFalse(SEL.broken_verdict(res, "z")["valid"])

    def test_reading_non_inferiority(self):
        rows = synth_rows({"ref": 0.9, "p": 0.9, "q": 0.3}, n=40, seed=2)
        res = SEL.select(rows, [], {}, seed=5, reference="ref")
        step = next(s for s in res["steps"] if s["step"] == "1b")
        self.assertEqual(step["eliminated"], ["q"])

    def test_paired_difference(self):
        rows = []
        for i in range(30):
            for d, c in (("a", 1.0), ("b", 0.0 if i % 3 else 1.0)):
                rows.append({"design": d, "qset": "throttle", "state": f"s{i}", "lang": "tr", "theme": "light",
                             "persona": "F", "rep": 0, "credit": c})
        p = SEL.paired_difference(rows, "a", "b", ("throttle",), seed=1)
        self.assertAlmostEqual(p["diff"], 20 / 30, places=3)
        self.assertTrue(p["separated"])
        self.assertEqual(p["n"], 30)

    def test_model_agreement(self):
        rows = [{"stim": "s", "persona": "F", "q": "dir", "rep": 0, "model": "h", "code": "up"},
                {"stim": "s", "persona": "F", "q": "dir", "rep": 0, "model": "s", "code": "up"},
                {"stim": "s", "persona": "F", "q": "kind", "rep": 0, "model": "h", "code": "flow"},
                {"stim": "s", "persona": "F", "q": "kind", "rep": 0, "model": "s", "code": "breakout"}]
        self.assertEqual(SEL.model_agreement(rows)["agree"], 0.5)

    def test_gates_and_waiver(self):
        comp = {"p": {"text_contrast": {"pass": False}, "touch": {"pass": True}},
                "q": {"text_contrast": {"pass": True}, "touch": {"pass": True}},
                "r": {"text_contrast": {"pass": True}, "touch": {"pass": True}}}
        st, waived = SEL.gate_status(comp, ["p", "q", "r"])
        self.assertFalse(st["p"]["pass"])
        self.assertEqual(waived, [])
        comp["q"]["touch"]["pass"] = comp["r"]["touch"]["pass"] = comp["p"]["touch"]["pass"] = False
        st, waived = SEL.gate_status(comp, ["p", "q", "r"])
        self.assertEqual(waived, ["touch"])
        self.assertTrue(st["q"]["pass"])


if __name__ == "__main__":
    unittest.main()
