//! Scenario presets S1–S19 (RESEARCH §4.2) as [`ScenarioSpec`] values.
//!
//! "Realistic" network baseline used where a scenario does not isolate a factor:
//! authority feed 20 ms + U(0, 20) ms, replica feed 80 ms + U(0, 60) ms, channel
//! 40 ms + U(0, 30) ms each way.

use crate::spec::{
    ChannelSpec, CorrectionKind, CorrectionSpec, Delay, FeedSpec, InvalidationSpec, PriceRepr,
    Race, RescaleSpec, ScenarioSpec, Shape, SteerStep, Window,
};

fn realistic(name: &str, description: &str) -> ScenarioSpec {
    ScenarioSpec {
        name: name.to_owned(),
        description: description.to_owned(),
        authority_feed: FeedSpec {
            delay: Delay::jitter(20.0, 20.0),
            ..FeedSpec::default()
        },
        replica_feed: FeedSpec {
            delay: Delay::jitter(80.0, 60.0),
            ..FeedSpec::default()
        },
        a2r: ChannelSpec {
            delay: Delay::jitter(40.0, 30.0),
            ..ChannelSpec::default()
        },
        r2a: ChannelSpec {
            delay: Delay::jitter(40.0, 30.0),
            ..ChannelSpec::default()
        },
        ..ScenarioSpec::default()
    }
}

fn hover(level: f64, amp: f64) -> Shape {
    Shape::Steer {
        plan: vec![SteerStep {
            level,
            amp,
            ..SteerStep::default()
        }],
        race: None,
    }
}

fn s2(name: &str, lag_ms: f64) -> ScenarioSpec {
    let mut s = realistic(
        name,
        &format!("replica feed lags by a constant {lag_ms} ms; protocol fast"),
    );
    s.replica_feed.delay = Delay::jitter(lag_ms, lag_ms * 0.05);
    s.drain_ms = s.drain_ms.max(lag_ms + 60_000.0);
    s
}

/// All presets, in table order (S2 has variants S2a 0.5 s, S2 5 s, S2c 60 s).
#[must_use]
#[allow(clippy::too_many_lines)]
#[allow(clippy::vec_init_then_push)]
pub fn presets() -> Vec<ScenarioSpec> {
    let mut v = Vec::new();
    v.push(realistic(
        "S0",
        "reference (not in RESEARCH): the realistic network baseline alone, free random walk",
    ));
    v.push(ScenarioSpec {
        name: "S1".into(),
        description: "ideal: no delay, no loss (identity; every approach should equal N)".into(),
        ..ScenarioSpec::default()
    });
    v.push(s2("S2a", 500.0));
    v.push(s2("S2", 5_000.0));
    v.push(s2("S2c", 60_000.0));
    {
        let mut s = realistic("S3", "protocol lags (3 s + U(0,1) s), feeds fast");
        s.replica_feed.delay = Delay::jitter(20.0, 20.0);
        s.a2r.delay = Delay::jitter(3_000.0, 1_000.0);
        s.r2a.delay = Delay::jitter(3_000.0, 1_000.0);
        v.push(s);
    }
    {
        let mut s = realistic(
            "S4",
            "protocol jitter + reordering + duplicates; a reconnect with resends",
        );
        for c in [&mut s.a2r, &mut s.r2a] {
            c.delay = Delay {
                base_ms: 50.0,
                jitter_ms: 400.0,
                spike_p: 0.02,
                spike_ms: 2_000.0,
            };
            c.reorder_p = 0.1;
            c.reorder_ms = 1_500.0;
            c.dup_p = 0.05;
            c.dup_ms = 1_000.0;
            c.dup_on_reconnect_p = 0.3;
        }
        s.disconnects = vec![Window {
            from_ms: 300_000.0,
            to_ms: 330_000.0,
        }];
        v.push(s);
    }
    {
        let mut s = realistic("S5", "replica feed reorders intra-bar ticks");
        s.ticks_per_bar = 60;
        s.replica_feed.reorder_p = 0.2;
        s.replica_feed.reorder_ms = 800.0;
        v.push(s);
    }
    {
        let mut s = realistic(
            "S6",
            "replica feed loss (QoS 0): random ticks, high-carrying ticks, bar-closing ticks; RSI near 70",
        );
        s.shape = Shape::Steer {
            plan: vec![SteerStep::default()],
            race: Some(Race::default()),
        };
        s.replica_feed.drop_p = 0.03;
        s.replica_feed.drop_high_tick_p = 0.3;
        s.replica_feed.drop_last_tick_p = 0.5;
        v.push(s);
    }
    {
        let mut s = realistic(
            "S7",
            "live-bar flicker: RSI oscillates around 70 inside the bar at 60 ticks/s",
        );
        s.ticks_per_bar = 3_600;
        s.live_bars = 8;
        s.shape = Shape::Steer {
            plan: vec![SteerStep {
                level: 70.0,
                amp: 2.5,
                osc: 6.0,
                ..SteerStep::default()
            }],
            race: None,
        };
        v.push(s);
    }
    {
        let mut s = realistic(
            "S8",
            "close race: the last tick before the close decides the cross; either side may miss it",
        );
        s.shape = Shape::Steer {
            plan: vec![SteerStep::default()],
            race: Some(Race::default()),
        };
        s.authority_feed.drop_last_tick_p = 0.25;
        s.replica_feed.drop_last_tick_p = 0.25;
        v.push(s);
    }
    {
        let mut s = realistic(
            "S9",
            "historical corrections: (a) in the evidence window, claim falls; (b) in the window, claim stands; (c) 1000 bars back; (d, added) a correction creates a claim",
        );
        s.history_bars = 1_200;
        s.corrections = vec![
            CorrectionSpec {
                at_bar: 6,
                kind: CorrectionKind::DropClaim,
            },
            CorrectionSpec {
                at_bar: 11,
                kind: CorrectionKind::KeepClaim,
            },
            CorrectionSpec {
                at_bar: 16,
                kind: CorrectionKind::Old { bars_back: 1_000 },
            },
            CorrectionSpec {
                at_bar: 21,
                kind: CorrectionKind::CreateClaim,
            },
            CorrectionSpec {
                at_bar: 26,
                kind: CorrectionKind::DropClaim,
            },
            CorrectionSpec {
                at_bar: 31,
                kind: CorrectionKind::CreateClaim,
            },
            CorrectionSpec {
                at_bar: 36,
                kind: CorrectionKind::KeepClaim,
            },
            CorrectionSpec {
                at_bar: 41,
                kind: CorrectionKind::CreateClaim,
            },
        ];
        v.push(s);
    }
    {
        let mut s = realistic(
            "S10",
            "split: whole history ×0.5 in the middle of live bar 8",
        );
        s.rescales = vec![RescaleSpec {
            at_bar: 8,
            factor: 0.5,
        }];
        v.push(s);
    }
    {
        let mut s = realistic(
            "S11",
            "replica reconnect: feed outage 30 s (REFRESH at reconnect, missed ticks never come) and protocol disconnect, RSI near 70",
        );
        s.shape = hover(70.0, 2.0);
        s.replica_feed.outages = vec![Window {
            from_ms: 240_000.0,
            to_ms: 270_000.0,
        }];
        s.disconnects = vec![Window {
            from_ms: 250_000.0,
            to_ms: 265_000.0,
        }];
        v.push(s);
    }
    {
        let mut s = ScenarioSpec {
            name: "S12".into(),
            description:
                "different compute anchor: authority 3000 bars, replica last 300 bars; no delay; RSI near 70"
                    .into(),
            history_bars: 3_000,
            shape: hover(70.0, 1.5),
            ..ScenarioSpec::default()
        };
        s.replica_feed.window_last = Some(300);
        v.push(s);
    }
    {
        let mut s = ScenarioSpec {
            name: "S12s".into(),
            description:
                "S12 with a short replica window (last 60 bars): the anchor difference reaches the predicate; no delay; RSI near 70"
                    .into(),
            history_bars: 3_000,
            shape: hover(70.0, 1.5),
            ..ScenarioSpec::default()
        };
        s.replica_feed.window_last = Some(60);
        v.push(s);
    }
    {
        let mut s = realistic(
            "S13",
            "duplicate delivery on the replica feed and on the protocol",
        );
        s.replica_feed.dup_p = 0.1;
        s.replica_feed.dup_ms = 2_000.0;
        s.a2r.dup_p = 0.1;
        s.a2r.dup_ms = 3_000.0;
        s.r2a.dup_p = 0.1;
        s.r2a.dup_ms = 3_000.0;
        v.push(s);
    }
    {
        let mut s = realistic(
            "S14",
            "authority feed faulty (glitches 1%, losses incl. bar-closing ticks), replica feed correct",
        );
        s.authority_feed.glitch_p = 0.02;
        s.authority_feed.glitch_size = 0.01;
        s.authority_feed.drop_p = 0.05;
        s.authority_feed.drop_last_tick_p = 0.3;
        v.push(s);
    }
    {
        let mut s = ScenarioSpec {
            name: "S15".into(),
            description:
                "permanent source difference: replica prices via f32, authority f64; no delay; RSI near 70"
                    .into(),
            shape: hover(70.0, 1.0),
            ..ScenarioSpec::default()
        };
        s.replica_feed.price_repr = PriceRepr::F32;
        v.push(s);
    }
    {
        let mut s = ScenarioSpec {
            name: "S15k".into(),
            description:
                "S15 on a knife edge: live prices unrounded, every steered bar closes with RSI within 1e-6 of 70; replica f32; no delay"
                    .into(),
            price_tick: 0.0,
            shape: Shape::Steer {
                plan: vec![SteerStep {
                    level: 70.0,
                    amp: 0.0,
                    noise: 0.0,
                    close_spread: 1e-6,
                    ..SteerStep::default()
                }],
                race: None,
            },
            ..ScenarioSpec::default()
        };
        s.replica_feed.price_repr = PriceRepr::F32;
        v.push(s);
    }
    {
        let mut s = realistic(
            "S16",
            "two-binding claim (both RSI above 70); replica feed of binding 1 lags 5 s",
        );
        s.bindings = 2;
        s.detector.pair = true;
        s.shape = hover(72.0, 2.0);
        let mut lag = s.replica_feed.clone();
        lag.delay = Delay::jitter(5_000.0, 250.0);
        s.replica_feed_overrides = vec![(1, lag)];
        v.push(s);
    }
    {
        let mut s = realistic(
            "S17",
            "load: 50 bindings, ~1000 ticks/s total, 10 extra studies per binding (500 in all)",
        );
        s.bindings = 50;
        s.ticks_per_bar = 1_200;
        s.extra_studies = 10;
        s.live_bars = 6;
        s.history_bars = 300;
        s.default_seeds = 20;
        v.push(s);
    }
    {
        let mut s = realistic(
            "S18",
            "finding and invalidation: cross above 70, then RSI hovers around 60 (invalidation appears and disappears on the live bar)",
        );
        s.detector.invalidation = Some(InvalidationSpec::default());
        s.shape = Shape::Steer {
            plan: vec![
                SteerStep {
                    level: 74.0,
                    bars: 3,
                    amp: 2.0,
                    ..SteerStep::default()
                },
                SteerStep {
                    level: 60.0,
                    bars: 5,
                    amp: 2.5,
                    osc: 4.0,
                    ..SteerStep::default()
                },
            ],
            race: None,
        };
        v.push(s);
    }
    {
        let mut s = realistic(
            "S19",
            "replica feed outage of 2 minutes (REFRESH at reconnect), then it continues; RSI near 70",
        );
        s.shape = hover(70.0, 2.0);
        s.replica_feed.outages = vec![Window {
            from_ms: 300_000.0,
            to_ms: 420_000.0,
        }];
        v.push(s);
    }
    v
}

/// Finds a preset by name (case-insensitive).
#[must_use]
pub fn find(name: &str) -> Option<ScenarioSpec> {
    presets()
        .into_iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
}
