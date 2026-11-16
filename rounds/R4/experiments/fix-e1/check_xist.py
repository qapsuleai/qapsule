"""FIX-E1/B1: compare the built-in XIST calendar (fixtures/calendar/xist.json) with local
real bars. Prints counts only (no symbols, prices or bar data); differences are printed
as dates (calendar facts). Usage:

    QAPSULE_LOCAL_BARS=/path/to/fixtures/local/bars python3 -I check_xist.py fixtures/calendar/xist.json

Full closures: weekdays from 2018-01-02 to the last daily bar with no daily bar in any
symbol. Half days: days whose latest hourly bar (over all symbols) opens before 15:00,
over the hourly range; the last day is skipped when its data is incomplete (fewer
symbols than usual or last 5-minute bar before the half-day close).
"""
import collections, datetime as dt, glob, json, os, sys

TZ = dt.timezone(dt.timedelta(hours=3))  # Europe/Istanbul, no DST since 2016-09
bars_dir = os.environ["QAPSULE_LOCAL_BARS"]
cal = json.load(open(sys.argv[1]))
hol = {dt.date.fromisoformat(d) for d in cal["holidays"]}
half = {dt.date.fromisoformat(h["date"]) for h in cal["half_days"]}

def lday(t):
    return dt.datetime.fromtimestamp(t / 1000, TZ)

daily = collections.Counter()
nsym = 0
for f in glob.glob(bars_dir + "/*.1day.adj.json"):
    nsym += 1
    for b in json.load(open(f))["data"]["bars"]:
        daily[lday(b["t"]).date()] += 1
first, last = dt.date(2018, 1, 2), max(daily)
weekend_bars = sum(1 for d in daily if d.weekday() >= 5)
data_closed, cal_closed, weekdays = set(), set(), 0
d = first
while d <= last:
    if d.weekday() < 5:
        weekdays += 1
        if daily[d] == 0:
            data_closed.add(d)
        if d in hol:
            cal_closed.add(d)
    d += dt.timedelta(1)

hourly_end = collections.defaultdict(list)
hourly_syms = 0
for f in glob.glob(bars_dir + "/*.1hour.adj.json"):
    hourly_syms += 1
    per = {}
    for b in json.load(open(f))["data"]["bars"]:
        x = lday(b["t"])
        per[x.date()] = max(per.get(x.date(), x), x)
    for day, end in per.items():
        hourly_end[day].append(end)
h_first, h_last = min(hourly_end), max(hourly_end)
incomplete = {day for day, ends in hourly_end.items() if len(ends) < hourly_syms * 0.9}
data_half = {day for day, ends in hourly_end.items()
             if max(ends).hour < 15 and day not in incomplete}
cal_half = {day for day in half if h_first <= day <= h_last and day.weekday() < 5
            and day not in incomplete}

print(f"symbols(daily)={nsym} range={first}..{last} weekdays={weekdays} weekend_bar_days={weekend_bars}")
print(f"full closures: data={len(data_closed)} calendar={len(cal_closed)} "
      f"only_data={sorted(map(str, data_closed - cal_closed))} "
      f"only_calendar={sorted(map(str, cal_closed - data_closed))}")
print(f"half days ({h_first}..{h_last}, {hourly_syms} symbols, incomplete days skipped: "
      f"{sorted(map(str, incomplete))}): data={len(data_half)} calendar={len(cal_half)} "
      f"only_data={sorted(map(str, data_half - cal_half))} "
      f"only_calendar={sorted(map(str, cal_half - data_half))}")
print(f"calendar totals: holidays={len(hol)} half_days={len(half)}")
