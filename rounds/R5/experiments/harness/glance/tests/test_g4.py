"""SPEC Ek G4: the three implementation fixes before M16 (voice passing, meaning colours per
theme, glyph body = most opaque prim)."""

import json
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))

import glance as G  # noqa: E402
from gl import compute as CP  # noqa: E402

SQ = [[8, 34], [40, 34], [40, 66], [8, 66]]


def glyph_scene(prims, bg=(255, 255, 255, 255)):
    return {"background": list(bg), "under": [], "over": [], "panels": [{"layers": [
        {"id": "glyph", "items": [{"hit": {"left": 8, "top": 34, "right": 40, "bottom": 66}}], "prims": prims}]}]}


def fill(color, pts=SQ):
    return {"prim": "fill", "color": list(color), "points": pts}


class VoiceTest(unittest.TestCase):
    """Ek G4-1: `--voice` reaches every address the harness opens and the records say it."""

    def test_default_is_the_product_default(self):
        self.assertIsNone(G.voice_of(types.SimpleNamespace(voice=None)))
        self.assertIsNone(G.voice_of(types.SimpleNamespace()))
        self.assertEqual(G.voice_of(types.SimpleNamespace(voice="marla"), {"name": "x"}), "marla")

    def test_shoot_plan_and_key_carry_the_voice(self):
        with tempfile.TemporaryDirectory() as t:
            run = Path(t)
            (run / "states.json").write_text(json.dumps({"states": [
                {"state": "live_event", "found": True, "step": 1, "candidates": ["c1"], "principal": "arkin"}]}))
            (run / "designs.json").write_text(json.dumps([{"name": "a", "face": "vadrel"}, {"name": "b", "look": "x"}]))
            a = types.SimpleNamespace(run=str(run), designs=str(run / "designs.json"), langs="tr", themes="light",
                                      kinds="face", paint="canvas", port=8601, qp="qp", web="web", voice="marla")
            with mock.patch.object(G, "node") as node, \
                    mock.patch.object(G.subprocess, "run", return_value=types.SimpleNamespace(returncode=1, stdout="")):
                G.cmd_shoot(a)
            node.assert_called_once()
            plan = json.loads((run / "shoot-plan.json").read_text())
            self.assertEqual({s["voice"] for s in plan["shots"]}, {"marla"})
            key = json.loads((run / "key.json").read_text())
            self.assertEqual({k["voice"] for k in key.values()}, {"marla"})

    def test_k52_passes_the_voice(self):
        with tempfile.TemporaryDirectory() as t:
            run = Path(t)
            (run / "designs.json").write_text(json.dumps([{"name": "a", "face": "vadrel"}]))
            a = types.SimpleNamespace(run=str(run), designs=str(run / "designs.json"), reps=1, charts=False,
                                      qp="qp", web="web", voice="marla")
            with mock.patch.object(G.subprocess, "run") as sp:
                G.cmd_k52(a)
            args = sp.call_args[0][0]
            self.assertEqual(args[args.index("--voice") + 1], "marla")
            a.voice = None
            with mock.patch.object(G.subprocess, "run") as sp:
                G.cmd_k52(a)
            self.assertNotIn("--voice", sp.call_args[0][0])

    def test_pages_set_the_address_parameter(self):
        # The browser side (no browser here): each script puts the voice into the address
        # and into its record.
        shoot = (HERE / "shoot.mjs").read_text()
        self.assertIn("q.set('voice', voice)", shoot)
        self.assertIn("voice = { requested:", shoot)
        k52 = (HERE / "k52.mjs").read_text()
        self.assertIn("voice && `voice=${encodeURIComponent(voice)}`", k52)
        self.assertIn("voice: voice || null", k52)
        self.assertIn("q.set('voice', plan.voice)", (HERE / "k52chart.mjs").read_text())


def face_rec(theme, validity, direction, colour, stage="born"):
    sc = glyph_scene([fill(colour)])
    return {"shot": {"id": f"{theme}-{validity}-{direction}", "theme": theme, "kind": "face"}, "target": "t",
            "engine": {"target_item": {"validity": validity, "direction": direction, "stage": stage},
                       "face_scenes": {"t": {"scene": sc}}},
            "dom": {"texts": [], "taps": []}}


class ThemeTest(unittest.TestCase):
    """Ek G4-2: meaning-colour pairs only within one theme; the gate per theme."""

    def recs(self):
        return [face_rec("light", "valid", "up", (0, 40, 140, 255)),
                face_rec("light", "valid", "down", (255, 170, 60, 255)),
                # dark: up and down nearly the same colour under deutan/protan (red/green)
                face_rec("dark", "valid", "up", (40, 170, 60, 255)),
                face_rec("dark", "valid", "down", (200, 60, 40, 255))]

    def test_pairs_never_mix_themes(self):
        sem = CP.semantic_colours(self.recs())
        self.assertEqual(sorted(sem), ["dark", "light"])
        g = CP.gates(self.recs())["colour"]
        self.assertEqual(len(g["pairs"]), 2)  # one up/down pair per theme, no light↔dark pair
        self.assertEqual(sorted(r["theme"] for r in g["pairs"]), ["dark", "light"])

    def test_gate_per_theme(self):
        g = CP.gates(self.recs())["colour"]
        self.assertTrue(g["themes"]["light"]["pass"])  # dark blue / light orange survive every condition
        self.assertFalse(g["themes"]["dark"]["pass"])  # red/green collapse
        self.assertFalse(g["pass"])

    def test_chart_pairs_per_theme(self):
        look = {"colors": {"light": {"up": "#00288cff", "down": "#ffaa3cff"},
                           "dark": {"up": "#28aa3cff", "down": "#c83c28ff"}}}

        def rec(theme):
            c = look["colors"][theme]
            return {"shot": {"id": theme, "theme": theme, "kind": "chart"}, "dom": {"texts": [], "taps": []},
                    "paints": {"scene": {"scale": 1, "bg": "#ffffffff", "prims": [
                        {"k": "rect", "x": 10, "y": 10, "w": 6, "h": 30, "c": c["up"]},
                        {"k": "rect", "x": 30, "y": 10, "w": 6, "h": 30, "c": c["down"]}]}}}
        g = CP.chart_gates([rec("light"), rec("dark")], look)["colour"]
        self.assertEqual(len(g["pairs"]), 2)
        self.assertTrue(all(r["a"] != r["b"] for r in g["pairs"]))  # no up↔up across themes
        self.assertTrue(g["themes"]["light"]["pass"])
        self.assertFalse(g["themes"]["dark"]["pass"])


class BodyTest(unittest.TestCase):
    """Ek G4-3: the glyph's colour is its most opaque prim, not the largest translucent part."""

    def test_opaque_line_beats_a_large_plate(self):
        plate = fill((120, 120, 200, 40), [[0, 0], [200, 0], [200, 100], [0, 100]])
        line = {"prim": "polyline", "color": [20, 20, 120, 255], "width": 2.0, "points": [[10, 50], [60, 50]]}
        _, g = CP.face_elements(glyph_scene([plate, line]))
        self.assertEqual(g["color"], (20, 20, 120, 255))
        self.assertGreater(g["ratio"], 3.0)
        self.assertLess(g["aux"]["ratio"], 1.5)  # the plate: reported, not the gate
        self.assertEqual(g["aux"]["color"], (120, 120, 200, 40))

    def test_ties_go_to_the_larger_area_and_knockouts_are_skipped(self):
        big = fill((200, 0, 0, 204), [[0, 0], [40, 0], [40, 10], [0, 10]])
        dot = fill((30, 30, 30, 204), [[0, 0], [4, 0], [4, 4], [0, 4]])
        knock = {"prim": "circle", "color": [255, 255, 255, 255], "r": 30}
        _, g = CP.face_elements(glyph_scene([big, dot, knock]))
        self.assertEqual(g["color"], (200, 0, 0, 204))
        self.assertNotIn("aux", g)

    def test_gate_uses_the_body(self):
        plate = fill((120, 120, 200, 40), [[0, 0], [200, 0], [200, 100], [0, 100]])
        line = {"prim": "polyline", "color": [20, 20, 120, 255], "width": 2.0, "points": [[10, 50], [60, 50]]}
        r = face_rec("light", "valid", "up", (0, 0, 0, 255))
        r["engine"]["face_scenes"]["t"]["scene"] = glyph_scene([plate, line])
        nt = CP.gates([r])["nontext_contrast"]
        self.assertTrue(nt["pass"])
        self.assertLess(nt["worst_aux"]["ratio"], 1.5)


if __name__ == "__main__":
    unittest.main()
