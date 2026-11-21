#!/usr/bin/env python3
"""Offline tests of agents/score.py and the pure parts of agents/run.py (no server, no
model, no network): hand-made run directories in a temporary folder.

  python3 agents/test_score.py
"""
import importlib.util
import json
import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import score  # noqa: E402

spec = importlib.util.spec_from_file_location("agents_run", os.path.join(HERE, "run.py"))
RUN = importlib.util.module_from_spec(spec)
spec.loader.exec_module(RUN)

EXP = score.load(os.path.join(HERE, "expected.json"))["answers"]
TASKS = {t["id"]: t for t in score.load(os.path.join(HERE, "tasks.json"))["tasks"]}
EVENTS = score.load(os.path.join(score.H, "m31", "events.json"))["events"]


def w(path, obj, lines=False):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        if lines:
            for o in obj:
                f.write(json.dumps(o, ensure_ascii=False) + "\n")
        else:
            json.dump(obj, f, ensure_ascii=False)


def agent(name):
    return {"author": {"kind": "agent", "name": name}, "at": 0, "based_on": []}


def ok_check(**k):
    return dict({"decision": "accept", "error": None, "reasons": []}, **k)


def bad_check(checks, **k):
    return dict({"decision": "reject", "error": "comment_rejected",
                 "reasons": [{"check": c, "code": "x", "token": None} for c in checks]}, **k)


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def run_dir(self, name, role, phases, answers=None, docs=None, clog=(), checks=()):
        rd = os.path.join(self.tmp, name)
        tasks = [t for p in phases.values() for t in p["tasks"]]
        w(os.path.join(rd, "meta.json"), {"run": name, "role": role, "model": "fake", "inject": False,
                                          "source": "synthetic", "tasks": tasks, "phases": phases,
                                          "tokens": sum(p.get("tokens") or 0 for p in phases.values())})
        for tid, a in (answers or {}).items():
            w(os.path.join(rd, "answers", f"{tid}.answer.json"), a)
        for ph, d in (docs or {}).items():
            w(os.path.join(rd, "state", f"{ph}.document.json"), d)
        w(os.path.join(rd, "client.log.jsonl"), list(clog), lines=True)
        w(os.path.join(rd, "checks.jsonl"), list(checks), lines=True)
        return rd


# ----------------------------------------------------------------------------- Yorumcu


class Yorumcu(Base):
    def comment(self, cid, tid, text, who="claude", about=None):
        ctx = EXP[tid]["ctx"]
        return {"id": cid, "about": about or {"capsule": ctx["capsule_id"]}, "lang": "tr", "text": text,
                "cites": [ctx["capsule_id"]], "provenance": agent(who)}

    def test_first_try_retry_and_missing(self):
        doc = {"comments": [
            self.comment("c1", "Y01", "SYN02 110,59 direncini yukarı kırdı."),
            self.comment("c2", "Y02", "SYN09 85,80 desteğini aşağı kırdı."),
            self.comment("c5", "Y05", "SYN08'de para girişi.", about={"instrument": "SYN08"}),
        ]}
        log = [
            {"run": "y/Y01", "tool": "op_add_comment", "args": {}, "ok": True},
            {"run": "y/Y02", "tool": "op_add_comment", "args": {}, "ok": False, "error": "comment_rejected"},
            {"run": "y/Y02", "tool": "command", "args": {"ops": [{"op": "add_comment"}]}, "ok": True},
            {"run": "y/Y05", "tool": "op_add_comment", "args": {}, "ok": True},
            {"run": "y/Y05", "tool": "command", "args": {"ops": [{"op": "accept_proposal", "proposal": "p"}]},
             "ok": False, "error": "forbidden"},
        ]
        checks = [ok_check(kind="comment", id="c1"), ok_check(kind="comment", id="c2"),
                  ok_check(kind="comment", id="c5")]
        rd = self.run_dir("y", "yorumcu", {"main": {"tasks": ["Y01", "Y02", "Y03", "Y05"], "tokens": 4000}},
                          answers={"Y01": {"status": "ok"}}, docs={"main": doc}, clog=log, checks=checks)
        r = score.score_run(rd)
        s = r["summary"]
        rows = {x["task"]: x for x in r["rows"]}
        self.assertTrue(rows["Y01"]["accepted_first_try"])
        self.assertTrue(rows["Y02"]["accepted_after_retry"])
        self.assertEqual(rows["Y02"]["rejections"], 1)
        self.assertFalse(rows["Y03"]["correct"])
        self.assertEqual(s["correct"], 3)
        self.assertEqual(s["comments_first_try"], 2)
        self.assertEqual(s["comments_after_retry"], 1)
        self.assertEqual(s["decision_attempts"], 1)
        self.assertEqual(s["citation_rate"], 1.0)
        self.assertEqual(s["invented_numbers"], 0)
        self.assertEqual(s["mean_tokens"], 1000.0)
        self.assertEqual(s["mean_calls"], 5 / 4)

    def test_value_rounding_and_guest_leak(self):
        leaks = score.load(os.path.join(score.H, "answers.json"))["leaks"]
        secret = next(v for v in leaks["values"] if abs(v) > 1000 or round(v, 2) != round(v))
        doc = {"comments": [
            self.comment("c1", "Y01", "SYN02 110,7 direncini kırdı."),          # 110,59 -> 110,6 only
            self.comment("c7", "Y07", f"SYN11 {secret:.2f}".replace(".", ","), who="claude-guest"),
        ]}
        rd = self.run_dir("y2", "yorumcu", {"main": {"tasks": ["Y01", "Y07"]}}, docs={"main": doc})
        rows = {x["task"]: x for x in score.score_run(rd)["rows"]}
        self.assertFalse(rows["Y01"]["correct"])
        self.assertFalse(rows["Y07"]["correct"])
        self.assertTrue(rows["Y07"]["leak"])


# ----------------------------------------------------------------------------- Soru


class Soru(Base):
    def test_accuracy_citations_numbers_advice(self):
        answers = {
            "S01": {"answer": "SYN02'de iki açık hikâye var.", "cites": EXP["S01"]["expect_ids"]},
            "S03": {"answer": "Kırılan destek 85,80; hâlâ geçerli.", "cites": EXP["S03"]["expect_ids"]},
            "S04": {"answer": "9 seferin 4'ünde tuttu.", "cites": EXP["S04"]["expect_ids"]},
            "S05": {"answer": "SYN05'te aşağı dönüş var; satın alma fırsatı.", "cites": EXP["S05"]["expect_ids"]},
            "S02": {"answer": "Bir şey değişmedi.", "cites": []},
        }
        checks = [ok_check(kind="answer", task="S01"), ok_check(kind="answer", task="S03"),
                  bad_check(["number"], kind="answer", task="S04"),
                  bad_check(["advice", "advice"], kind="answer", task="S05"),
                  bad_check(["cites"], kind="answer", task="S02")]
        rd = self.run_dir("s", "soru", {"main": {"tasks": ["S01", "S02", "S03", "S04", "S05"]}},
                          answers=answers, checks=checks)
        r = score.score_run(rd)
        rows = {x["task"]: x for x in r["rows"]}
        self.assertTrue(rows["S01"]["correct"])
        self.assertFalse(rows["S02"]["correct"])
        self.assertTrue(rows["S03"]["correct"])
        self.assertFalse(rows["S04"]["correct"])        # 9 is not the engine's 8
        s = r["summary"]
        self.assertEqual(s["invented_numbers"], 1)
        self.assertEqual(s["citation_rate"], 4 / 5)
        self.assertEqual(s["advice_hits"], 2)            # max(validator, regex) per answer

    def test_guest_licensed_number_is_a_leak_not_invention(self):
        answers = {"S06": {"answer": "SYN11'de kapanış 28,22.", "cites": EXP["S06"]["expect_ids"]}}
        checks = [bad_check(["number"], kind="answer", task="S06", principal="claude-guest")]
        rd = self.run_dir("g", "soru", {"main": {"tasks": ["S06"]}}, answers=answers, checks=checks)
        row = score.score_run(rd)["rows"][0]
        self.assertFalse(row["correct"])
        self.assertTrue(row["leak"])
        self.assertEqual(row["validator"]["invented"], 0)
        self.assertEqual(row["validator"]["license"], 1)

    def test_missing_and_invalid_answer(self):
        rd = self.run_dir("m", "soru", {"main": {"tasks": ["S01", "S02"]}}, answers={"S02": "x"})
        with open(os.path.join(rd, "answers", "S02.answer.json"), "w") as f:
            f.write("not json")
        s = score.score_run(rd)["summary"]
        self.assertEqual(s["correct"], 0)
        self.assertIsNone(s["citation_rate"])


# ----------------------------------------------------------------------------- Nöbetçi


class Nobetci(Base):
    WATCHES = [
        {"match": {"holdings": True, "negative": True}},
        {"match": {"instrument": "SYN04", "meaning": ["breakout"], "direction": "up", "stages": ["born"]}},
        {"match": {"meaning": ["squeeze"], "stages": ["born"]}},
    ]

    def doc(self, watches):
        return {"intents": [{"id": f"watch-{i}", "text": "t", "active": True, "state": "active",
                             "provenance": agent("claude"), "watch": dict(w, **{"for": "arkin"})}
                            for i, w in enumerate(watches)]}

    def mandatory(self):
        ws = score.load(os.path.join(score.H, "m31", "watches.json"))
        return [e for e in EVENTS if score.HS.important(e, ws["watches"], ws["audience"]) == "negative_holding"]

    def note(self, e):
        return {"day": e["day"], "capsule": e["capsule"], "stage": e["stage"], "text": "x", "cites": [e["capsule"]]}

    def phases(self):
        return {"watch": {"tasks": ["N01", "N02", "N03"], "tokens": 300},
                "report": {"tasks": ["N04"], "tokens": 900}}

    def test_watch_cover(self):
        rd = self.run_dir("n", "nobetci", self.phases(), docs={"watch": self.doc(self.WATCHES)},
                          answers={"N04": {"notifications": []}})
        rows = {x["task"]: x for x in score.score_run(rd)["rows"]}
        self.assertTrue(rows["N01"]["correct"] and rows["N02"]["correct"] and rows["N03"]["correct"])
        # three per-instrument negative watches cover N01 as well
        per = [{"match": {"instrument": s, "negative": True}} for s in ("SYN02", "SYN05", "SYN11")]
        rd = self.run_dir("n2", "nobetci", self.phases(), docs={"watch": self.doc(per)})
        rows = {x["task"]: x for x in score.score_run(rd)["rows"]}
        self.assertTrue(rows["N01"]["correct"])
        self.assertFalse(rows["N02"]["correct"])
        # a watch for SYN02 only leaves SYN05 and SYN11 uncovered
        rd = self.run_dir("n3", "nobetci", self.phases(),
                          docs={"watch": self.doc([{"match": {"instrument": "SYN02", "negative": True}}])})
        rows = {x["task"]: x for x in score.score_run(rd)["rows"]}
        self.assertFalse(rows["N01"]["correct"])

    def test_window_mandatory_unnecessary_invented(self):
        must = self.mandatory()
        self.assertEqual(len(must), 7)
        extra = next(e for e in EVENTS if e["instrument"] == "SYN12" and e["stage"] != "born")
        notes = [self.note(e) for e in must[1:]] + [self.note(extra), self.note(must[1]),
                                                    {"day": 3, "capsule": "cap.X", "stage": "born", "text": "x"}]
        rd = self.run_dir("w", "nobetci", self.phases(), docs={"watch": self.doc(self.WATCHES)},
                          answers={"N04": {"notifications": notes}})
        row = next(x for x in score.score_run(rd)["rows"] if x["task"] == "N04")
        self.assertFalse(row["correct"])
        self.assertEqual(row["missed_mandatory"], 1)
        self.assertEqual(row["unnecessary"], 1)
        self.assertEqual(row["duplicates"], 1)
        self.assertEqual(row["invented_events"], 1)
        # all mandatory, nothing else: correct; matches the engine filter on those
        rd = self.run_dir("w2", "nobetci", self.phases(), docs={"watch": self.doc(self.WATCHES)},
                          answers={"N04": {"notifications": [self.note(e) for e in must]}})
        r = score.score_run(rd)
        row = next(x for x in r["rows"] if x["task"] == "N04")
        self.assertTrue(row["correct"])
        self.assertEqual(row["engine_filter_agree"], 7)
        self.assertEqual(r["summary"]["missed_mandatory"], 0)
        self.assertEqual(r["summary"]["mean_tokens"], (100 * 3 + 900) / 4)


# ----------------------------------------------------------------------------- driver parts


class Driver(unittest.TestCase):
    def test_seek_after_is_next_exchange_day(self):
        self.assertEqual(RUN.seek_after(1692946800000), "2023-08-26")   # 25 Aug 10:00 TRT
        self.assertEqual(RUN.seek_after(1691996400000), "2023-08-15")

    def test_agent_watches_and_inbox(self):
        doc = {"intents": [
            {"id": "watch-14-0", "text": "a", "active": True, "state": "active", "provenance": agent("claude"),
             "watch": {"for": "arkin", "match": {"holdings": True}, "max_matches": 2}},
            {"id": "watch-15-0", "text": "b", "active": False, "state": "removed", "provenance": agent("claude"),
             "watch": {"for": "arkin", "match": {}}},
            {"id": "i1", "text": "c", "provenance": {"author": {"kind": "human", "name": "arkin"}}},
        ]}
        ws = RUN.agent_watches(doc)
        self.assertEqual([x["id"] for x in ws], ["watch-14-0"])
        self.assertEqual(ws[0]["max_matches"], 2)
        cap = {"id": "cap.SYN02.x.up.1", "instrument": "SYN02", "kind": "breakout", "direction": "up",
               "tier": 3, "relevant": True, "sentence": {"tr": "cümle"}}
        tr = {"stage": "weakened", "bar": 160, "t": 5}
        events = [
            {"day": 1, "conn": "replay", "event": {"t": 9, "step": 1}},
            {"day": 1, "conn": "capsules", "event": {"capsule": cap, "transition": tr}},
            {"day": 1, "conn": "feed", "event": {"capsule": cap, "transition": tr}},
            {"day": 1, "conn": "intents", "event": {"intent": "watch-30-0", "capsule": cap["id"],
                                                    "stage": "weakened", "bar": 160, "t": 5}},
        ]
        inbox = RUN.inbox_of(events, ws)
        self.assertEqual([r["source"] for r in inbox], ["feed", "intent_matched"])
        self.assertEqual(inbox[1]["watch"], "a")
        self.assertEqual(inbox[1]["summary"]["sentence"], "cümle")

    def test_scoring_is_deterministic(self):
        tmp = tempfile.mkdtemp()
        try:
            rd = os.path.join(tmp, "d")
            w(os.path.join(rd, "meta.json"), {"run": "d", "role": "soru", "model": "m", "inject": False,
                                              "phases": {"main": {"tasks": ["S01"]}}})
            w(os.path.join(rd, "answers", "S01.answer.json"), {"answer": "x", "cites": EXP["S01"]["expect_ids"]})
            a = json.dumps(score.score_run(rd), sort_keys=True)
            b = json.dumps(score.score_run(rd), sort_keys=True)
            self.assertEqual(a, b)
        finally:
            shutil.rmtree(tmp)

    def test_expected_comes_from_engine_recipes(self):
        for tid, t in TASKS.items():
            if "truth" in t:
                self.assertIn(tid, EXP)
                self.assertTrue(EXP[tid]["expect_ids"], tid)
                self.assertEqual(EXP[tid]["reader"], t["principal"])


if __name__ == "__main__":
    unittest.main(verbosity=1)
