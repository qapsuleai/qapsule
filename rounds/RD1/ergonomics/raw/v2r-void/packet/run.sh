#!/bin/sh
# Runs your message file (one JSON line per message: {"conn": "...", "msg": {...}})
# against a fresh copy of the document and prints the messages the authority sends to
# your connections. Every run starts from the same fresh state.
QP_ERGO_VARIANT=renamed exec python3 /Users/arkns/Works/Mchart/qapsule/rounds/RD1/ergonomics/harness/ergo.py run "$1"
