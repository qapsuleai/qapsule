"""SPEC Ek G5: the meaning-colour gate from the look record's roles (per theme), not from the
glyph body (which, after Ek G4, can be the text-coloured outline of every mark)."""

import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))

from gl import compute as CP  # noqa: E402

SQ = [[8, 34], [40, 34], [40, 66], [8, 66]]

LOOK = {"glyph": {"face_px": 32.0}, "colors": {
    "light": {"bg": "#ffffffff", "text": "#111111ff", "up": "#00288cff", "down": "#ff8c1eff",
              "neutral": "#808080ff", "valid": "#111111ff", "invalid": "#b0b0b0ff", "closed": "#b0b0b0ff",
              "grid": "#eeeeeeff", "s2": "#00288cff"},
    "dark": {"bg": "#000000ff", "text": "#eeeeeeff", "up": "#28aa3cff", "down": "#c83c28ff",
             "neutral": "#808080ff", "valid": "#eeeeeeff", "invalid": "#555555ff", "closed": "#555555ff"}}}


def rec(theme, body, edge=(17, 17, 17, 255)):
    """A face whose glyph is a filled body plus an opaque outline in the text colour (siluet)."""
    prims = [{"prim": "fill", "color": list(body[:3]) + [200], "points": SQ},
             {"prim": "polyline", "color": list(edge), "width": 2.0, "points": [[8, 34], [40, 34]]}]
    sc = {"background": [255, 255, 255, 255], "under": [], "over": [], "panels": [{"layers": [
        {"id": "glyph", "items": [{"hit": {"left": 8, "top": 34, "right": 40, "bottom": 66}}], "prims": prims}]}]}
    return {"shot": {"id": f"{theme}-{body}", "theme": theme, "kind": "face"}, "target": "t",
            "engine": {"target_item": {"validity": "valid", "direction": "up", "stage": "born"},
                       "face_scenes": {"t": {"scene": sc}}},
            "dom": {"texts": [], "taps": []}}


class RoleGateTest(unittest.TestCase):
    def test_pairs_from_roles_per_theme(self):
        g = CP.role_colour_gate(LOOK, ["light", "dark"])
        self.assertEqual(g["rule"], "Ek G5 (roles)")
        self.assertTrue(all(r["theme"] in ("light", "dark") for r in g["pairs"]))
        # invalid = closed (same colour) and valid = text: a deliberate share, listed, not counted
        sh = {(s["theme"], s["a"], s["b"]) for s in g["shared"]}
        self.assertIn(("light", "invalid", "closed"), sh)
        self.assertNotIn(("light", "invalid", "closed"), {(r["theme"], r["a"], r["b"]) for r in g["pairs"]})
        # dark up/down are red/green: the dark theme fails under protan/deutan, light passes that pair
        dark = {(r["a"], r["b"]): r for r in g["themes"]["dark"]["pairs"]}
        self.assertFalse(dark[("up", "down")]["pass"])
        light = {(r["a"], r["b"]): r for r in g["themes"]["light"]["pairs"]}
        self.assertTrue(light[("up", "down")]["pass"])
        self.assertFalse(g["pass"])

    def test_threshold_follows_mark_size(self):
        g = CP.role_colour_gate(LOOK, ["light"], sizes={"light": 14})
        self.assertTrue(all(r["threshold"] == 15.0 for r in g["pairs"]))
        g = CP.role_colour_gate(LOOK, ["light"])  # face_px 32 from the record
        self.assertTrue(all(r["threshold"] == 10.0 for r in g["pairs"]))

    def test_mark_roles_from_glyph_prims(self):
        recs = [rec("light", (0, 40, 140)), rec("light", (255, 140, 30))]
        used = CP.mark_roles(recs, LOOK)
        # the outline is the text colour (= valid here: every role of that RGB), the bodies
        # up/down; series roles (s2) never count
        self.assertEqual(used["light"], ["down", "text", "up", "valid"])

    def test_gates_use_roles_not_the_outline(self):
        # Every mark's most opaque prim is the same text-coloured outline: the scene table sees
        # one colour (no colour-coded pair); the role gate still measures up/down/... per theme.
        recs = [rec("light", (0, 40, 140)), rec("light", (255, 140, 30))]
        g = CP.gates(recs, LOOK)
        self.assertEqual(g["colour"]["rule"], "Ek G5 (roles)")
        self.assertIn("text", g["colour"]["themes"]["light"]["roles"])
        self.assertTrue(any(r["a"] == "up" and r["b"] == "down" for r in g["colour"]["pairs"]))
        self.assertIn("colour_scene", g)
        # without a look record the scene table remains the gate (before Ek G5)
        self.assertNotIn("rule", CP.gates(recs)["colour"])


if __name__ == "__main__":
    unittest.main()
