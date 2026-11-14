# Geçersiz örnekler

Her dosya kendi tipinin üretilen şemasına göre **reddedilmelidir**.

| Dosya | Neden geçersiz |
|-------|----------------|
| `client_msg.no_type.json` | type missing |
| `client_msg.unknown_field.json` | clients cannot claim authorship |
| `client_msg.bad_surface.json` | unknown surface kind |
| `client_msg.command_no_id.json` | id required |
| `client_msg.bad_mode.json` | unknown write mode |
| `client_msg.query_unknown_kind.json` | unknown query kind |
| `client_msg.subscribe_no_topic.json` | topic required |
| `client_msg.input_bad_gesture.json` | unknown gesture |
| `client_msg.attend_no_to.json` | target principal required |
| `client_msg.clock_string.json` | time is epoch ms |
| `server_msg.bad_status.json` | unknown ack status |
| `server_msg.error_bad_code.json` | unknown error code |
| `server_msg.welcome_missing_profile.json` | profile required |
| `op.add_study_no_inputs.json` | inputs required |
| `op.set_params_string.json` | parameters are numbers |
| `change_op.semantic_op.json` | semantic ops are not changes |
| `query.list_bad_collection.json` | unknown collection |
| `log_entry.no_changes.json` | changes required |
| `inbound_line.no_conn.json` | conn required |
| `outbound_line.client_msg.json` | outbound carries server messages |
