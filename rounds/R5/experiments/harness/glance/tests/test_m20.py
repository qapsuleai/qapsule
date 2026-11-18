"""R5/M20 harness additions (SPEC Ek G7): e_who key, numeral scan, capsule-depth sets."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
import glance as G  # noqa: E402
from gl import compute as CP  # noqa: E402
from gl import keys as K  # noqa: E402


class WhoKey(unittest.TestCase):
    def test_open_trust_decides(self):
        item = {"trust": "server"}
        self.assertEqual(G.who_key({"open": {"trust": "Bu cihazda sınandı"}}, item), "device")
        self.assertEqual(G.who_key({"open": {"trust": "Checking on this device"}}, item), "device")
        self.assertEqual(G.who_key({"open": {"trust": "Sunucu doğruladı"}}, {"trust": "device"}), "server")
        self.assertEqual(G.who_key({"open": {}}, item), "server")

    def test_capsule_record_keys(self):
        rec = {"engine": {"target_item": {"validity": "valid", "stage": "confirmed", "trust": "server"}, "replay_t": 0},
               "open": {"trust": "Bu cihazda sınandı"},
               "evidence_key": [{"family": "price"}, {"family": "volume"}]}
        kr = G.key_record(rec, "cap_evidence")
        k = K.keys_for("cap_evidence", kr)
        self.assertEqual(k["e_rests"], {"price", "volume"})
        self.assertEqual(k["e_who"], {"device"})


class Numerals(unittest.TestCase):
    def test_symbol_and_entitlement_excepted(self):
        self.assertEqual(CP.digit_words("Kilitli: flow/SYN03 yetkisi · SYN21"), [])
        self.assertEqual(CP.digit_words("Eşik · 114,27"), ["114,27"])

    def test_clock_and_data_names_skipped(self):
        rec = {"shot": {"principal": "visitor", "id": "x", "kind": "capsule"},
               "dom": {"texts": [{"t": "2023-06-19 10:00", "id": "clock"}, {"t": "bars SYN28 1d (feed default)", "id": ""}]},
               "evidence": [{"requires": "SYN28", "rests_on": "bars SYN28 1d (feed default)"}], "paints": {}}
        self.assertTrue(CP.visitor_digits([rec])["pass"])
        rec["dom"]["texts"].append({"t": "Değer 159,86", "id": ""})
        self.assertFalse(CP.visitor_digits([rec])["pass"])


if __name__ == "__main__":
    unittest.main()
