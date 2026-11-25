# Borç taban çizgisi

üretilmiştir: `scripts/arch-check --debt-md` (kaynak `scripts/arch-baseline.json` ve
`scripts/arch-config.json` istisnaları). Elle düzenlemeyin.

Kural `docs/ENGINEERING.md` §1.4: borç görünürdür ve büyümez. `scripts/arch-check` taban
çizgisindeki değerden ya da sayıdan **fazlasına** hata verir (mandal); yeni kalem ancak
`--write-baseline --allow-new` ile ve gerekçesiyle girer.

## İstisna istekleri (Arkin'in kararını bekliyor)

| No | Kural | Kalem | Gerekçe | Durum |
|---|---|---|---|---|
| İ1 | katman | `qp-source` → `alan` | §2 qp-source'u uyarlayıcıya koyar; içeriği veri düzlemidir (FeedStore §18.4, SourceProvider, MemoryProvider, sentetik piyasa, yeniden oynatma takvimi, canlı bar kurma) ve oturum onu otoritenin parçası olarak kullanır. Dış dünyaya bakan kısım yerel dosya okuma (local.rs, file.rs). Saf taşımayla ayrılamaz: Session::new_inner gövdesi MemoryProvider::from_init'i çağırır (RG1/PLAN §3.1). | onaylandı (K85, 2026-11-22) |
| İ3 | `crate.lines` | `qp-authority` | RF1 sonrası otorite çekirdeği: komut yolu, günlük, iki dünya ve as_reader takası ile birlikte kapsül yaşamı, iddia/itiraz/yorum ve canlı yol aynı crate'te kaldı (bayt eşitliği için; aralarında döngü ve yeniden giriş zinciri var — rounds/RF1/PLAN.md §9 B1–B5). Çekirdek donuktur; yeni iş alt sistem sözleşmesiyle kendi crate'inde doğar. Kalan borç: kapsül yaşamı, iddia/itiraz/yorum ve canlı yolun alt sisteme ayrılması (aday: RS1'den sonra; rounds/RF1/DECISION.md cevap 2). | onaylandı (K85, 2026-11-22); tavan 12744 |

## Özet

| Kural | Ne | Hedef | Kalem |
|---|---|---|---|
| `dir.files` | Klasörde 15'ten çok kaynak dosya | konu klasörleri; ≤ 15 | 3 |
| `doc.size` | Belge 1.500 satır ya da 60 KB'ı aşıyor | konuya göre böl, dizin bırak | 2 |
| `file.soft` | Dosya 600 satırı aşıyor (kesin sınır 800 değil) | konuya göre alt modül; ≤ 600 | 32 |
| `fn.lines` | Fonksiyon 80 satırı aşıyor (RG1'de fonksiyon bölünmedi) | adlandırılmış adımlara böl; ≤ 80 | 173 |
| `lint.allow` | Mevcut lint ihlali öğe başına `#[allow]` ile susturuldu (RG1/L) | ihlali düzelt, izni kaldır | 22 |
| `test.file` | Test dosyası 800 satırı aşıyor (M5 kesildi) | konuya göre böl; ≤ 800 | 6 |

## `dir.files` — Klasörde 15'ten çok kaynak dosya

Hedef: konu klasörleri; ≤ 15.

| Kalem | Değer |
|---|---|
| `crates/qp-session/tests` | 39 |
| `crates/qp-wasm/tests` | 20 |
| `crates/qp-compute/tests` | 16 |

## `doc.size` — Belge 1.500 satır ya da 60 KB'ı aşıyor

Hedef: konuya göre böl, dizin bırak.

| Kalem | Değer |
|---|---|
| `docs/PROTOCOL.md` | 2956 |
| `docs/matriks/INDICATORS.md` | 1203 |

## `file.soft` — Dosya 600 satırı aşıyor (kesin sınır 800 değil)

Hedef: konuya göre alt modül; ≤ 600.

| Kalem | Değer |
|---|---|
| `crates/qp-capsule/src/outcome.rs` | 762 |
| `crates/qp-authority/src/capsule/dispute.rs` | 758 |
| `crates/qp-plane/src/plane.rs` | 753 |
| `crates/qp-source/src/replay.rs` | 752 |
| `crates/qp-matriks/src/trade.rs` | 725 |
| `crates/qp-ops/src/gops.rs` | 723 |
| `crates/qp-schema/src/protocol/ops.rs` | 712 |
| `crates/qp-shellcore/src/shell/bench.rs` | 708 |
| `crates/qp-shellcore/src/shell/paint_gpu.rs` | 688 |
| `crates/qp-read/src/interact.rs` | 685 |
| `crates/qp-authority/src/capsule/comments.rs` | 672 |
| `crates/qp-voice/src/marla.rs` | 669 |
| `crates/qp-live/src/config.rs` | 660 |
| `crates/qp-wasm/src/bind/shell.rs` | 658 |
| `crates/qp-session/src/conform.rs` | 645 |
| `crates/qp-shellcore/src/shell/paint_canvas.rs` | 644 |
| `crates/qp-ops/src/select.rs` | 643 |
| `crates/qp-capsule/src/realize/en.rs` | 641 |
| `crates/qp-authority/src/capsule/reader.rs` | 639 |
| `crates/qp-shellcore/src/gallery.rs` | 631 |
| `crates/qp-authority/src/capsule/claims.rs` | 630 |
| `crates/qp-host/src/demo.rs` | 630 |
| `crates/qp-capsule/src/engine.rs` | 624 |
| `crates/qp-voice/src/selvi.rs` | 624 |
| `crates/qp-capsule/src/realize/tr.rs` | 623 |
| `crates/qp-shellcore/src/shell/metrics.rs` | 621 |
| `crates/qp-host/src/live.rs` | 616 |
| `crates/qp-shellcore/src/shell/inbound.rs` | 611 |
| `crates/qp-shellcore/src/shell.rs` | 610 |
| `crates/qp-compute/src/sweep/share.rs` | 605 |
| `crates/qp-glyph/src/rozet.rs` | 604 |
| `crates/qp-plane/src/values.rs` | 603 |

## `fn.lines` — Fonksiyon 80 satırı aşıyor (RG1'de fonksiyon bölünmedi)

Hedef: adlandırılmış adımlara böl; ≤ 80.

| Kalem | Değer |
|---|---|
| `crates/qp-designs/src/stories/pavka/extend.rs::extend` | 592 |
| `crates/qp-ops/src/gops.rs::expand_grammar` | 571 |
| `crates/qp-grammar/src/compile/scene.rs::compile` | 562 |
| `crates/qp-read/src/query.rs::query` | 365 |
| `crates/qp-authority/src/authority/session/command.rs::command` | 300 |
| `crates/qp-host/src/live_trades/compare.rs::compare_trades` | 294 |
| `crates/qp-ops/src/apply.rs::apply_change` | 289 |
| `crates/qp-host/src/demo.rs::demo` | 277 |
| `crates/qp-live/src/conn/session.rs::session` | 272 |
| `crates/qp-voice/src/marla/head.rs::core` | 260 |
| `crates/qp-glyph/src/siluet.rs::draw` | 251 |
| `crates/qp-shellcore/src/shell/inbound.rs::on_message` | 245 |
| `crates/qp-capsule/src/engine/run.rs::run_bar` | 240 |
| `crates/qp-capsule/src/comment/check.rs::validate` | 239 |
| `crates/qp-designs/src/open/tesir/view.rs::view` | 230 |
| `crates/qp-designs/src/open/nobra.rs::view` | 217 |
| `crates/qp-live/src/mock/serve.rs::serve_connection` | 210 |
| `crates/qp-cli/src/cmd/replay.rs::replay_cmd` | 199 |
| `crates/qp-glyph/src/rozet.rs::draw` | 198 |
| `crates/qp-shellcore/src/shell/bench.rs::bench_tick` | 198 |
| `crates/qp-voice/src/selvi/clause.rs::clause` | 196 |
| `crates/qp-authority/src/authority/session/dispatch.rs::handle_resolved` | 193 |
| `crates/qp-voice/src/marla/why.rs::why_numbers` | 190 |
| `crates/qp-read/src/interact.rs::interact` | 186 |
| `crates/qp-mcp/src/surfaces/query/act.rs::act` | 185 |
| `crates/qp-host/src/demo/setup.rs::live_setup` | 182 |
| `crates/qp-viewer/src/tape.rs::run_viewer` | 178 |
| `crates/qp-designs/src/stories/korda/scene.rs::scene` | 177 |
| `crates/qp-ops/src/canon.rs::canon_view` | 176 |
| `crates/qp-voice/src/marla/since.rs::latest_clause` | 175 |
| `crates/qp-host/src/live_trades/export.rs::export_trade_bars` | 169 |
| `crates/qp-compute/src/address.rs::expr` | 168 |
| `crates/qp-host/src/demo/http.rs::connection` | 166 |
| `crates/qp-read/src/query/view.rs::input` | 166 |
| `crates/qp-host/src/live/compare.rs::compare` | 165 |
| `crates/qp-authority/src/capsule/dispute.rs::dispute` | 162 |
| `crates/qp-read/src/capsules.rs::capsules_query` | 160 |
| `crates/qp-designs/src/stories/korda/draw.rs::lane` | 159 |
| `crates/qp-source/src/livebar/push.rs::push` | 159 |
| `crates/qp-authority/src/authority/session/worlds.rs::propose_two_worlds` | 157 |
| `crates/qp-replica/src/tape.rs::run_with` | 157 |
| `crates/qp-authority/src/authority/session/create.rs::build_with_keys` | 154 |
| `crates/qp-ops/src/expand/undo.rs::expand_undo` | 153 |
| `crates/qp-authority/src/authority/session/events.rs::entry_events_in` | 151 |
| `crates/qp-authority/src/capsule/project.rs::capsule_topic_lines_in` | 150 |
| `crates/qp-authority/src/capsule/settled.rs::capsule_settled` | 149 |
| `crates/qp-mcp/src/surfaces/task/explain.rs::explain` | 149 |
| `crates/qp-authority/src/capsule/run.rs::run_capsules` | 143 |
| `crates/qp-ops/src/canon/origin.rs::resolve_origins` | 142 |
| `crates/qp-voice/src/selvi/ledger.rs::ledger` | 142 |
| `crates/qp-host/src/live/inspect.rs::inspect` | 141 |
| `crates/qp-designs/src/motion/frame.rs::change_frame` | 136 |
| `crates/qp-replica/src/replica/judge.rs::judge` | 133 |
| `crates/qp-host/src/live/record.rs::record` | 131 |
| `crates/qp-mcp/src/surfaces/task.rs::whats_new` | 131 |
| `crates/qp-mcp/src/surfaces/task/instrument.rs::instrument` | 131 |
| `crates/qp-authority/src/authority/session/worlds.rs::direct_two_worlds` | 130 |
| `crates/qp-grammar/src/compile/draw/series.rs::lines` | 129 |
| `crates/qp-host/src/demo/hub.rs::handle` | 128 |
| `crates/qp-mcp/src/surfaces/query/find.rs::find_stories` | 127 |
| `crates/qp-host/src/live_trades/compare.rs::source_table` | 126 |
| `crates/qp-mcp/src/surfaces/query/ask.rs::ask` | 125 |
| `crates/qp-shellcore/src/drive/viewer_less.rs::viewer_less` | 125 |
| `crates/qp-authority/src/capsule/run.rs::capsule_changes` | 124 |
| `crates/qp-viewer/src/viewer/updates.rs::viewer_compute` | 123 |
| `crates/qp-mcp/src/surfaces/mixed.rs::overview` | 122 |
| `crates/qp-authority/src/data/live/eval.rs::bar_command` | 121 |
| `crates/qp-capsule/src/realize/tr.rs::event_clause` | 121 |
| `crates/qp-designs/src/stories/korda/draw.rs::histogram` | 121 |
| `crates/qp-host/src/live/compare.rs::load_rest_with` | 121 |
| `crates/qp-read/src/bindq.rs::check_fact` | 121 |
| `crates/qp-shellcore/src/drive.rs::shell_less` | 121 |
| `crates/qp-designs/src/transitions/nabiz/overlay.rs::overlay` | 120 |
| `crates/qp-authority/src/capsule/public.rs::translate` | 119 |
| `crates/qp-read/src/interact.rs::tap_at` | 119 |
| `crates/qp-host/src/livehost/runtime.rs::apply` | 118 |
| `crates/qp-host/src/mcp_serve/host.rs::post` | 117 |
| `crates/qp-mcp/src/surfaces/query.rs::shape` | 117 |
| `crates/qp-read/src/interact/scenes.rs::transition` | 116 |
| `crates/qp-read/src/interact/style.rs::set_style` | 116 |
| `crates/qp-shellcore/src/gallery/build_impl.rs::shots` | 116 |
| `crates/qp-authority/src/authority/session/intake.rs::resolve_selectors` | 115 |
| `crates/qp-voice/src/selvi/detail.rs::d1_compact` | 114 |
| `crates/qp-mcp/src/surfaces/query/find.rs::find_instrument` | 113 |
| `crates/qp-ops/src/canon/layer.rs::check_channels` | 108 |
| `crates/qp-ops/src/select/resolve.rs::resolve_kind` | 108 |
| `crates/qp-render/src/retained.rs::paint` | 108 |
| `crates/qp-designs/src/faces/zemre.rs::view` | 106 |
| `crates/qp-matriks/src/feed/ingest.rs::on_symbol` | 106 |
| `crates/qp-shellcore/src/shell/input.rs::pointer` | 106 |
| `crates/qp-shellcore/src/shell/paint_canvas.rs::prim` | 106 |
| `crates/qp-authority/src/authority/session/worlds.rs::accept` | 105 |
| `crates/qp-read/src/interact/scenes.rs::depth_view` | 105 |
| `crates/qp-source/src/livebar/push.rs::push_trade` | 105 |
| `crates/qp-authority/src/data/live/eval.rs::eval_conditions_live` | 104 |
| `crates/qp-mcp/src/surfaces/mixed.rs::watch` | 104 |
| `crates/qp-voice/src/selvi_glossary.rs::entry` | 104 |
| `crates/qp-live/src/config.rs::from_lookup` | 103 |
| `crates/qp-authority/src/capsule/claims.rs::after_bars` | 102 |
| `crates/qp-mcp/src/surfaces/query/find.rs::find_capsules` | 102 |
| `crates/qp-ops/src/canon/layer.rs::check_layer` | 102 |
| `crates/qp-ops/src/locate.rs::walk` | 102 |
| `crates/qp-mcp/src/surfaces/query/capsule.rs::detail_parts` | 101 |
| `crates/qp-ops/src/expand/op.rs::expand_op` | 101 |
| `crates/qp-authority/src/capsule/story.rs::stories_query` | 100 |
| `crates/qp-designs/src/transitions/zumlak.rs::frame` | 100 |
| `crates/qp-host/src/mcp_serve/http.rs::handle` | 100 |
| `crates/qp-ops/src/expand/undo.rs::expand_remove` | 100 |
| `crates/qp-shellcore/src/shell/metrics.rs::metrics_rows` | 100 |
| `crates/qp-ops/src/summary.rs::summarize_doc` | 99 |
| `crates/qp-ops/src/select/sites.rs::op_sites` | 98 |
| `crates/qp-viewer/src/viewer/open.rs::open_scene_with` | 98 |
| `crates/qp-grammar/src/compile/layer/rows.rs::pair_rows` | 97 |
| `crates/qp-ops/src/select/rows.rs::layer_rows` | 97 |
| `crates/qp-authority/src/capsule/project.rs::as_reader` | 96 |
| `crates/qp-grammar/src/compile/draw/series.rs::areas` | 96 |
| `crates/qp-mcp/src/tools.rs::generate` | 96 |
| `crates/qp-ops/src/expand.rs::check_bind` | 96 |
| `crates/qp-authority/src/capsule/detail.rs::capsule_query_with` | 95 |
| `crates/qp-authority/src/capsule/project.rs::project_changes` | 95 |
| `crates/qp-designs/src/faces/fotolay.rs::view` | 95 |
| `crates/qp-ops/src/locate.rs::walk_branches` | 95 |
| `crates/qp-voice/src/marla_glossary.rs::en` | 95 |
| `crates/qp-authority/src/authority/session/intake.rs::handle_raw` | 94 |
| `crates/qp-ops/src/expand/describe.rs::summarize` | 94 |
| `crates/qp-capsule/src/realize/plan.rs::new` | 93 |
| `crates/qp-grammar/src/compile/emphasis.rs::resolve_emphasis` | 93 |
| `crates/qp-grammar/src/compile/terms.rs::mark` | 93 |
| `crates/qp-host/src/live/rest.rs::trades` | 93 |
| `crates/qp-read/src/query/view.rs::screen_info` | 93 |
| `crates/qp-replayhost/src/replayhost/control.rs::host_control` | 93 |
| `crates/qp-capsule/src/realize/en.rs::event_clause` | 92 |
| `crates/qp-shellcore/src/shell.rs::new` | 92 |
| `crates/qp-shellcore/src/shell/inbound.rs::on_frame` | 92 |
| `crates/qp-source/src/replay.rs::new` | 92 |
| `crates/qp-grammar/src/compile/layer/rows.rs::series_rows` | 91 |
| `crates/qp-grammar/src/present/depth/card.rs::chart_view_r3` | 91 |
| `crates/qp-host/src/live_trades/export.rs::price_steps` | 91 |
| `crates/qp-ops/src/select/rows.rs::series_rows` | 91 |
| `crates/qp-mcp/src/surfaces/mixed/text.rs::capsule_card` | 90 |
| `crates/qp-voice/src/marla_glossary.rs::tr` | 90 |
| `crates/qp-authority/src/capsule/settled.rs::capsule_dispute` | 89 |
| `crates/qp-mcp/src/server_tools.rs::render` | 89 |
| `crates/qp-mcp/src/surfaces/task/watch.rs::watch` | 89 |
| `crates/qp-render/src/spikes/vello_cpu.rs::rasterize` | 89 |
| `crates/qp-authority/src/capsule/comments.rs::comment_events` | 88 |
| `crates/qp-grammar/src/compile/draw/marks.rs::anchored_text` | 88 |
| `crates/qp-grammar/src/compile/layer/resolve.rs::resolve_constants` | 88 |
| `crates/qp-live/src/mock/issuer.rs::handle_token_request` | 88 |
| `crates/qp-replayhost/src/replayhost/setup.rs::expand_init` | 88 |
| `crates/qp-shellcore/src/shell/scene.rs::scene` | 88 |
| `crates/qp-authority/src/capsule/project.rs::project_doc` | 87 |
| `crates/qp-compute/src/lanes.rs::bollinger` | 87 |
| `crates/qp-host/src/live/inspect.rs::inspect_trades` | 87 |
| `crates/qp-live/src/mqtt.rs::decode_body` | 87 |
| `crates/qp-viewer/src/viewer.rs::slice` | 87 |
| `crates/qp-read/src/interact.rs::move_chart` | 86 |
| `crates/qp-capsule/src/realize/tr.rs::remark` | 85 |
| `crates/qp-cli/src/cmd/serve.rs::serve_replay` | 85 |
| `crates/qp-authority/src/authority/session/presence.rs::subscribe` | 84 |
| `crates/qp-capsule/src/realize/en.rs::remark` | 84 |
| `crates/qp-host/src/livehost.rs::new` | 84 |
| `crates/qp-matriks/src/feed/trades.rs::on_trade` | 84 |
| `crates/qp-mcp/src/server.rs::handle_message` | 84 |
| `crates/qp-mcp/src/surfaces/task.rs::tools` | 84 |
| `crates/qp-voice/src/selvi/holds.rs::d_holds` | 84 |
| `crates/qp-facts/src/squeeze_bandwidth.rs::eval` | 83 |
| `crates/qp-host/src/mcp_serve.rs::serve` | 83 |
| `crates/qp-plane/src/plane.rs::apply_event` | 83 |
| `crates/qp-render/src/spikes/vello_gpu.rs::render_async` | 83 |
| `crates/qp-ops/src/select/rows.rs::finding_rows` | 82 |
| `crates/qp-mcp/src/surfaces/query/find_records.rs::find_changes` | 81 |
| `crates/qp-render/src/spikes/vello_gpu.rs::encode` | 81 |

## `lint.allow` — Mevcut lint ihlali öğe başına `#[allow]` ile susturuldu (RG1/L)

Hedef: ihlali düzelt, izni kaldır.

| Kalem | Değer |
|---|---|
| `qp-compute` | 27 |
| `qp-authority` | 25 |
| `qp-live` | 23 |
| `qp-host` | 21 |
| `qp-ops` | 20 |
| `qp-mcp` | 17 |
| `qp-shellcore` | 17 |
| `qp-capsule` | 14 |
| `qp-designs` | 14 |
| `qp-voice` | 12 |
| `qp-grammar` | 11 |
| `qp-read` | 11 |
| `qp-facts` | 9 |
| `qp-source` | 5 |
| `qp-replica` | 4 |
| `qp-viewer` | 4 |
| `qp-glyph` | 3 |
| `qp-matriks` | 3 |
| `qp-replayhost` | 3 |
| `qp-cli` | 2 |
| `qp-plane` | 1 |
| `qp-render` | 1 |

## `test.file` — Test dosyası 800 satırı aşıyor (M5 kesildi)

Hedef: konuya göre böl; ≤ 800.

| Kalem | Değer |
|---|---|
| `crates/qp-source/tests/data_plane.rs` | 1135 |
| `crates/qp-session/tests/grammar.rs` | 1076 |
| `crates/qp-capsule/tests/comment.rs` | 981 |
| `crates/qp-session/tests/non_interference.rs` | 954 |
| `crates/qp-designs/tests/capsule_depth.rs` | 845 |
| `crates/qp-designs/tests/pavka_ref/mod.rs` | 827 |

