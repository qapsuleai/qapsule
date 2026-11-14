<!--
RD1/T6e — prompt for the "LLM writes the grammar" arm.
Send everything below the line "=== PROMPT START ===" as the single user message
(no system prompt). Record per sample: model id, temperature (state if default),
input/output token counts, wall-clock duration (request sent → last byte), raw text.
-->
=== PROMPT START ===
You are the presentation agent of a charting engine. A user opened a finding on a desktop
surface at "chart" depth. Your job: write the visual-grammar recipe (a `View` object) that
the engine will render to present this finding. The renderer draws exactly what the View
says; it adds nothing on its own.

## The document (what you can reference)

Instrument: THYAO (BIST), daily bars (interval `1d`), display time zone Europe/Istanbul.

Series (ids are exact; no other series exist):

| id      | kind                  | definition                                   | outputs |
|---------|-----------------------|----------------------------------------------|---------|
| `px`    | bars (OHLCV), inline  | 200 daily bars, bar index 0..199             | o, h, l, c, v |
| `rsi14` | study `rsi`           | inputs `["px"]`, params `{period: 14}`       | rsi (range 0..100) |
| `macd`  | study `macd`          | inputs `["px"]`, params `{fast:12, slow:26, signal:9}` | macd, signal, histogram |
| `bb20`  | study `bollinger`     | inputs `["px"]`, params `{period:20, k:2}`   | middle, upper, lower, bandwidth |

Bar timestamps (epoch milliseconds, UTC): bar 0 = 1735776000000, bar 150 = 1753920000000,
bar 199 (last) = 1759881600000. Bars are trading days only (weekends/holidays skipped).

## The finding to present

```json
{
  "id": "fnd-overbought",
  "claim": "THYAO günlükte RSI(14) aşırı alım bölgesine girdi; momentum güçlü.",
  "facts": ["fact-rsi-cross70"],
  "confidence": 0.65,
  "invalidation": {"kind": "threshold", "series": "rsi14", "op": "cross_below", "value": 70.0}
}
```

Its fact (computed by the engine):

```json
{
  "id": "fact-rsi-cross70",
  "kind": "cross_above",
  "series": "rsi14",
  "bar": 150,
  "t": 1753920000000,
  "text": "RSI(14) 70'i yukarı kesti, bar 150.",
  "evidence": {"level": 70.0, "prev": 69.35847609603053, "value": 72.83959265032422}
}
```

## Grammar notes

- A View has one horizontal scale `x` shared by all panels and one vertical scale per
  panel in `y` (`y[i]` is the scale of panel `i`). Every panel used by a layer must have
  a scale in `y`.
- Panel 0 is the main price panel.
- `x.kind = "time"`: `x.domain`, if given, is `[from, to]` in epoch milliseconds UTC.
  Omit `domain` for automatic (all bars).
- A layer binds a `mark` to a `series` id. Constant marks (e.g. a horizontal `rule`)
  have no `series`; their position is given in `encoding`, e.g. `{"y": "70"}`.
- `encoding` maps a visual channel to a field name of the series or to a constant; all
  values are strings, e.g. `{"y": "rsi", "color": "#7e57c2"}`.
- Layers are drawn in order.

## JSON Schema of `View` (generated from the engine's types; authoritative)

```json
{
  "$defs": {
    "Layer": {
      "additionalProperties": false,
      "description": "One layer of a view: a mark bound to a series.",
      "properties": {
        "encoding": {
          "additionalProperties": {
            "type": "string"
          },
          "default": {},
          "description": "Encoding: visual channel -> field or constant, e.g. `{\"y\": \"v\", \"color\": \"#888\"}`.",
          "type": "object"
        },
        "mark": {
          "$ref": "#/$defs/Mark",
          "description": "Visual mark."
        },
        "panel": {
          "default": 0,
          "description": "Panel index (0 = main price panel).",
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "series": {
          "description": "Series id; absent for constant marks (e.g. a rule).",
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "mark"
      ],
      "type": "object"
    },
    "Mark": {
      "description": "Visual mark of a layer.",
      "oneOf": [
        {
          "const": "candle",
          "description": "Candlesticks (needs bars).",
          "type": "string"
        },
        {
          "const": "line",
          "description": "Polyline.",
          "type": "string"
        },
        {
          "const": "area",
          "description": "Filled area.",
          "type": "string"
        },
        {
          "const": "bar",
          "description": "Vertical bars.",
          "type": "string"
        },
        {
          "const": "rule",
          "description": "Horizontal or vertical reference line.",
          "type": "string"
        },
        {
          "const": "point",
          "description": "Points.",
          "type": "string"
        },
        {
          "const": "text",
          "description": "Text label.",
          "type": "string"
        }
      ]
    },
    "Scale": {
      "additionalProperties": false,
      "description": "A scale mapping data values to a visual axis.",
      "properties": {
        "domain": {
          "description": "Fixed domain `[min, max]`; automatic when absent.",
          "items": {
            "format": "double",
            "type": "number"
          },
          "maxItems": 2,
          "minItems": 2,
          "type": [
            "array",
            "null"
          ]
        },
        "kind": {
          "$ref": "#/$defs/ScaleKind",
          "description": "Scale type."
        }
      },
      "required": [
        "kind"
      ],
      "type": "object"
    },
    "ScaleKind": {
      "description": "Scale type.",
      "oneOf": [
        {
          "const": "time",
          "description": "Time axis.",
          "type": "string"
        },
        {
          "const": "linear",
          "description": "Linear numeric.",
          "type": "string"
        },
        {
          "const": "log",
          "description": "Logarithmic numeric.",
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "A presentation recipe written in the visual grammar.",
  "properties": {
    "display_tz": {
      "description": "Display time zone override (IANA); display only, see [`Document::display_tz`].",
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "description": "Unique id within the document.",
      "type": "string"
    },
    "layers": {
      "description": "Layers, drawn in order.",
      "items": {
        "$ref": "#/$defs/Layer"
      },
      "type": "array"
    },
    "title": {
      "description": "Optional title.",
      "type": [
        "string",
        "null"
      ]
    },
    "x": {
      "$ref": "#/$defs/Scale",
      "description": "Horizontal scale."
    },
    "y": {
      "description": "Vertical scale per panel, indexed by `Layer::panel`.",
      "items": {
        "$ref": "#/$defs/Scale"
      },
      "type": "array"
    }
  },
  "required": [
    "id",
    "x",
    "y",
    "layers"
  ],
  "title": "View",
  "type": "object"
}
```

## Task

Write the `View` that presents finding `fnd-overbought` at "chart" depth on desktop, so
that a user immediately sees the evidence for the claim. Use the id
`present-fnd-overbought`.

Output ONLY the `View` JSON object: no prose, no explanation, no Markdown code fences.
