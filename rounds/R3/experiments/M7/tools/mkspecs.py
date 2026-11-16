import json, copy, sys
S, out = sys.argv[1], sys.argv[2]
s9 = json.load(open(S))
a = copy.deepcopy(s9)
a["name"] = "AUD1"
a["description"] = "RP2 AUDIT AUD1 (rounds/RP2/AUDIT.md): S9 + steer 70 + close race, reorder/dup authority feed, lagging reordering replica feed, noisy channel, 8 corrections, split x0.5 at bar 30"
a["live_bars"] = 50
a["shape"] = {"kind": "steer", "plan": [{"level": 70, "bars": 1000, "amp": 2, "osc": 3, "close_spread": 2, "noise": 0.15}],
              "race": {"margin": 1, "before_close_ms": 300, "cross_p": 0.5}}
af = a["authority_feed"]
af["delay"] = {"base_ms": 20, "jitter_ms": 60, "spike_p": 0.02, "spike_ms": 800}
af.update({"reorder_p": 0.15, "reorder_ms": 600, "dup_p": 0.05, "dup_ms": 500, "drop_last_tick_p": 0.05})
rf = a["replica_feed"]
rf["delay"] = {"base_ms": 1500, "jitter_ms": 2500, "spike_p": 0.03, "spike_ms": 4000}
rf.update({"reorder_p": 0.25, "reorder_ms": 1500, "dup_p": 0.05, "dup_ms": 1000, "drop_last_tick_p": 0.1})
ch = {"delay": {"base_ms": 300, "jitter_ms": 1200, "spike_p": 0.05, "spike_ms": 3000},
      "reorder_p": 0.2, "reorder_ms": 2000, "dup_p": 0.1, "dup_ms": 1500, "dup_on_reconnect_p": 0.3}
a["a2r"] = copy.deepcopy(ch)
a["r2a"] = copy.deepcopy(ch)
a["corrections"] = [{"at_bar": 5, "kind": "drop_claim"}, {"at_bar": 9, "kind": "create_claim"},
                    {"at_bar": 13, "kind": "keep_claim"}, {"at_bar": 17, "kind": "drop_claim"},
                    {"at_bar": 22, "kind": "create_claim"}, {"at_bar": 27, "kind": "old", "bars_back": 800},
                    {"at_bar": 33, "kind": "create_claim"}, {"at_bar": 38, "kind": "drop_claim"}]
a["rescales"] = [{"at_bar": 30, "factor": 0.5}]
json.dump(a, open(out + "/aud1.json", "w"), indent=2)

b = copy.deepcopy(a)
b["name"] = "AUD2"
b["description"] = "RP2 AUDIT AUD2: AUD1 + replica feed outage 540-780 s without REFRESH, channel cut 600-640 s, corrections (bars 10, 11, 12, 25) and x2 split (bar 11) inside the outage"
b["replica_feed"]["outages"] = [{"from_ms": 540000, "to_ms": 780000}]
b["replica_feed"]["refresh_on_reconnect"] = False
b["disconnects"] = [{"from_ms": 600000, "to_ms": 640000}]
b["corrections"] = [{"at_bar": 10, "kind": "drop_claim"}, {"at_bar": 11, "kind": "create_claim"},
                    {"at_bar": 12, "kind": "keep_claim"}, {"at_bar": 25, "kind": "drop_claim"}]
b["rescales"] = [{"at_bar": 11, "factor": 2}]
json.dump(b, open(out + "/aud2.json", "w"), indent=2)

c = copy.deepcopy(a)
c["name"] = "AUD3"
c["description"] = "RP2 AUDIT AUD3: AUD1 + replica prices f32, 250 ms conflation (renumbered seq), close race margin 0.2"
c["replica_feed"]["price_repr"] = "f32"
c["replica_feed"]["conflate_ms"] = 250
c["shape"]["race"]["margin"] = 0.2
json.dump(c, open(out + "/aud3.json", "w"), indent=2)
