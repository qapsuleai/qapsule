"""M18 story-chart keys and the chart plot extent (deterministic parts)."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from gl import compute as CP  # noqa: E402
from gl import keys as K  # noqa: E402


def row(i, born, closed=None, outcome=None):
    r = {"id": f"c{i}", "born_bar": born, "last_bar": closed or 100}
    if closed is not None:
        r.update(closed_bar=closed, outcome=outcome, validity="closed")
    else:
        r["validity"] = "valid"
    return r


class KeysTest(unittest.TestCase):
    def test_count_buckets(self):
        self.assertEqual(K.count_key([]), "0")
        self.assertEqual(K.count_key([1] * 5), "1-9")
        self.assertEqual(K.count_key([1] * 50), "30-99")
        self.assertEqual(K.count_key([1] * 212), "100+")

    def test_held_share(self):
        rs = [row(1, 1, 5, "realized"), row(2, 2, 6, "invalidated"), row(3, 3, 7, "retracted")]
        self.assertEqual(K.held_key(rs), "half")  # retracted is not a closing that counts
        self.assertEqual(K.held_key([row(1, 1)]), "none_closed")
        self.assertEqual(K.held_key([row(1, 1, 5, "invalidated"), row(2, 1, 5, "expired")]), "few")

    def test_live_columns_and_margins(self):
        rng = (0, 100)
        self.assertEqual(K.live_keys([row(1, 10, 20, "realized")], rng), {"none"})
        self.assertEqual(K.live_keys([row(1, 60)], rng), {"C"})
        # 24 % of the range: within 10 % of a column width of the 25 % line -> not scorable
        self.assertEqual(K.live_keys([row(1, 24)], rng), set())
        self.assertEqual(K.live_keys([row(1, 0), row(2, 99)], rng), {"A", "D"})
        # born before the visible range: not scorable
        self.assertEqual(K.live_keys([row(1, -5)], rng), set())

    def test_chart_set_keys(self):
        rec = {"item": {"kind": "flow"}, "stories": [row(1, 90)], "stories_visible": [row(1, 90)], "range": [0, 100],
               "now_t": 0}
        k = K.keys_for("chart", rec)
        self.assertEqual(k, {"c_recent": {"1-9"}, "c_held": {"none_closed"}, "c_live": {"D"}, "c_evid": {"flow"}})


class PlotTest(unittest.TestCase):
    def test_plot_ignores_the_price_tag(self):
        rec = {"scene_rect": [0, 100, 390, 300], "shot": {"theme": "light"},
               "look_record": {"colors": {"light": {"up": "#00ff00ff", "down": "#ff0000ff"}}},
               "paints": {"scene": {"scale": 1, "bg": "#ffffffff", "prims": [
                   {"k": "rect", "x": 10, "y": 50, "w": 4, "h": 20, "c": "#00ff00ff"},
                   {"k": "rect", "x": 300, "y": 60, "w": 4, "h": 20, "c": "#ff0000ff"},
                   {"k": "rect", "x": 318, "y": 90, "w": 72, "h": 16, "c": "#00ff00ff"},
                   {"k": "text", "x": 324, "y": 102, "t": "36.32", "font": "11px QpMono", "c": "#ffffffff"}]}}}
        self.assertEqual(CP.chart_plot(rec), [10, 304, 100, 400])


if __name__ == "__main__":
    unittest.main()
