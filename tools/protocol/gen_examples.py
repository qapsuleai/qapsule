#!/usr/bin/env python3
"""Writes fixtures/protocol/{valid,invalid}/<type>.<name>.json: example messages per
protocol type. Valid ones must validate against the generated JSON Schema of <type>,
invalid ones must be rejected (checked by scripts/verify.sh step `protocol`)."""
import json, os

T0 = 1760000000000
V, I = {}, {}
def v(t, n, x): V[f"{t}.{n}"] = x
def i(t, n, x, why): I[f"{t}.{n}"] = {"_why": why, **x} if False else x; WHY[f"{t}.{n}"] = why
WHY = {}

hello = {"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"watch","screen":[198,242],"input":["touch","crown"]}}
v("client_msg","hello",hello)
v("client_msg","command",{"type":"command","id":"c1","ops":[{"op":"add_study","study":"rsi","inputs":["px"],"params":{"period":7},"show":{"view":"main","with":"rsi14"}}]})
v("client_msg","command_propose",{"type":"command","id":"c2","mode":"propose","reason":"RSI 70'i kesti","base":4,"ops":[{"op":"add_finding","claim":"Momentum güçlü.","facts":["fact-rsi-cross70"],"confidence":0.6,"invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":70.0},"capsule":"Momentum güçlü"}]})
v("client_msg","decide",{"type":"command","id":"c3","ops":[{"op":"accept_proposal","proposal":"prop-6","note":"ok"}]})
v("client_msg","query",{"type":"query","id":"q1","query":{"kind":"study","series":"rsi14","range":{"last":50},"reduce":"max"}})
v("client_msg","subscribe",{"type":"subscribe","id":"s1","topic":{"kind":"condition","condition":{"kind":"compare","left":"sma20","op":"cross_above","right":"sma50"}}})
v("client_msg","unsubscribe",{"type":"unsubscribe","id":"u1","sub":"s1"})
v("client_msg","input",{"type":"input","id":"i1","gesture":{"kind":"zoom","view":"main","factor":2.0,"anchor_bar":199}})
v("client_msg","attend",{"type":"attend","id":"a1","directive":{"to":"arkin","steps":[{"target":{"view":"main","panel":1,"series":"rsi14","from_bar":150,"to_bar":150},"say":"RSI burada 70'i kesti."}]}})
v("client_msg","clock",{"type":"clock","now":T0})
i("client_msg","no_type",{"protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}},"type missing")
i("client_msg","unknown_field",{"type":"command","id":"c1","ops":[],"mode":"auto","author":"arkin"},"clients cannot claim authorship")
i("client_msg","bad_surface",{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"fridge"}},"unknown surface kind")
i("client_msg","command_no_id",{"type":"command","ops":[{"op":"remove","id":"bb20"}]},"id required")
i("client_msg","bad_mode",{"type":"command","id":"c","mode":"force","ops":[]},"unknown write mode")
i("client_msg","query_unknown_kind",{"type":"query","id":"q","query":{"kind":"pixels"}},"unknown query kind")
i("client_msg","subscribe_no_topic",{"type":"subscribe","id":"s"},"topic required")
i("client_msg","input_bad_gesture",{"type":"input","id":"i","gesture":{"kind":"pinch","view":"main"}},"unknown gesture")
i("client_msg","attend_no_to",{"type":"attend","id":"a","directive":{"steps":[]}},"target principal required")
i("client_msg","clock_string",{"type":"clock","now":"2026-11-14"},"time is epoch ms")

prof={"depth":"sentence","series_data":False,"max_points":0,"max_glyphs":1,"speech":False}
lim={"max_message_bytes":1048576,"max_ops":64,"max_bars":10000,"max_subscriptions":32,"max_string":4096}
perm={"read":True,"write":True,"propose":True,"decide":True,"attend":True,"feed":False}
v("server_msg","welcome",{"type":"welcome","protocol":"qp/0","doc":"doc-thyao","principal":"arkin","role":"owner","permissions":perm,"seq":4,"epoch":0,"profile":prof,"limits":lim})
v("server_msg","ack_proposed",{"type":"ack","id":"c7","status":"proposed","seq":6,"proposal":"prop-6","created":["fnd-6-0","cap-6-1"]})
v("server_msg","result_reduced",{"type":"result","id":"q1","seq":0,"data":{"kind":"reduced","series":"rsi14","reduce":"max","bars":50,"values":{"rsi":{"value":78.11504477458953,"bar":154,"t":1754265600000}}}})
v("server_msg","notify_change",{"type":"notify","sub":"s1","event":{"kind":"change","seq":5,"at":T0,"author":"arkin","cause":"c1","changes":[{"op":"set_capsule_state","capsule":"cap-overbought","state":"rejected"}]}})
v("server_msg","notify_attention",{"type":"notify","sub":"me","event":{"kind":"attention","directive":{"id":"att-1","to":"arkin","from":"scout","at":T0,"steps":[{"target":{"series":"rsi14","from_bar":150,"to_bar":150}}]}}})
v("server_msg","error",{"type":"error","id":"c4","error":{"code":"not_found","message":"series `rsi` does not exist","path":"/ops/0/show/with","did_you_mean":["rsi14"],"hint":"Use an existing series id."}})
i("server_msg","bad_status",{"type":"ack","id":"c","status":"ok"},"unknown ack status")
i("server_msg","error_bad_code",{"type":"error","error":{"code":"oops","message":"x"}},"unknown error code")
i("server_msg","welcome_missing_profile",{"type":"welcome","protocol":"qp/0","doc":"d","principal":"a","role":"owner","permissions":perm,"seq":0,"epoch":0,"limits":lim},"profile required")

v("op","reject_finding",{"op":"reject_finding","finding":"fnd-overbought","note":"hacim teyidi yok"})
v("op","add_narrative",{"op":"add_narrative","about":"fnd-overbought","steps":[{"at_ms":0,"target":{"series":"px"},"say":"Fiyat yükseliyor."}]})
v("op","undo",{"op":"undo"})
i("op","add_study_no_inputs",{"op":"add_study","study":"rsi"},"inputs required")
i("op","set_params_string",{"op":"set_params","series":"rsi14","params":{"period":"14"}},"parameters are numbers")

v("change_op","insert_layer",{"op":"insert_layer","view":"main","index":5,"layer":{"mark":"line","series":"series-1-0","panel":1,"encoding":{}}})
v("change_op","pop_ledger",{"op":"pop_ledger"})
i("change_op","semantic_op",{"op":"add_study","study":"rsi","inputs":["px"]},"semantic ops are not changes")

v("query","at",{"kind":"at","series":"px","bar":150})
v("query","present",{"kind":"present","surface":"watch"})
v("query","describe",{"kind":"describe"})
i("query","list_bad_collection",{"kind":"list","collection":"bars"},"unknown collection")

v("log_entry","undo",{"seq":3,"at":T0,"author":"arkin","cause":"c3","request":[{"op":"undo"}],"changes":[{"op":"delete","id":"sma20"}],"undoes":1,"summary":"undo 1"})
i("log_entry","no_changes",{"seq":3,"at":T0,"author":"arkin","summary":"x"},"changes required")

v("inbound_line","hello",{"conn":"a","msg":hello})
i("inbound_line","no_conn",{"msg":hello},"conn required")
v("outbound_line","error",{"conn":"","msg":{"type":"error","error":{"code":"parse","message":"line is not JSON"}}})
i("outbound_line","client_msg",{"conn":"a","msg":hello},"outbound carries server messages")

os.chdir(os.path.join(os.path.dirname(__file__), "..", ".."))
for kind, d in (("valid", V), ("invalid", I)):
    os.makedirs(f"fixtures/protocol/{kind}", exist_ok=True)
    for name, x in d.items():
        with open(f"fixtures/protocol/{kind}/{name}.json", "w", encoding="utf-8") as f:
            json.dump(x, f, ensure_ascii=False, indent=1); f.write("\n")
with open("fixtures/protocol/invalid/README.md","w",encoding="utf-8") as f:
    f.write("# Geçersiz örnekler\n\nHer dosya kendi tipinin üretilen şemasına göre **reddedilmelidir**.\n\n| Dosya | Neden geçersiz |\n|-------|----------------|\n")
    for n in I: f.write(f"| `{n}.json` | {WHY[n]} |\n")
print(len(V),"valid",len(I),"invalid")
