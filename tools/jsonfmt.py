"""JSON text in Prettier's layout, so files our tools write pass the lint job's
prettier check without a Node step (LINT-01).

Objects are always expanded (Prettier keeps an expanded object expanded); an array
stays on one line when it fits in 80 columns, else one item per line (numbers fill
the lines). Usage: from jsonfmt import dumps; open(p, "w").write(dumps(data))
"""

import json

WIDTH = 80


def _scalar(value):
    return json.dumps(value, ensure_ascii=False)


def _flat(value):
    """One-line form, or None when the value cannot be on one line."""
    if isinstance(value, dict):
        return "{}" if not value else None
    if isinstance(value, list):
        parts = [_flat(v) for v in value]
        if any(p is None for p in parts):
            return None
        return "[" + ", ".join(parts) + "]"
    return _scalar(value)


def _is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _fill(value, indent):
    pad = " " * (indent + 2)
    rows, row = [], ""
    for i, item in enumerate(value):
        word = _scalar(item) + ("," if i < len(value) - 1 else "")
        if row and len(pad) + len(row) + 1 + len(word) > WIDTH:
            rows.append(pad + row)
            row = word
        else:
            row = row + " " + word if row else word
    rows.append(pad + row)
    return "[\n" + "\n".join(rows) + "\n" + " " * indent + "]"


def _fmt(value, indent, column, tail):
    """value starts at `column`; `tail` characters (a comma) follow it."""
    pad = " " * (indent + 2)
    if isinstance(value, dict):
        if not value:
            return "{}"
        items = list(value.items())
        lines = []
        for i, (key, item) in enumerate(items):
            comma = "," if i < len(items) - 1 else ""
            head = pad + _scalar(str(key)) + ": "
            lines.append(head + _fmt(item, indent + 2, len(head), len(comma)) + comma)
        return "{\n" + "\n".join(lines) + "\n" + " " * indent + "}"
    if isinstance(value, list):
        flat = _flat(value)
        if flat is not None and column + len(flat) + tail <= WIDTH:
            return flat
        if value and all(_is_number(v) for v in value):
            return _fill(value, indent)
        lines = []
        for i, item in enumerate(value):
            comma = "," if i < len(value) - 1 else ""
            lines.append(pad + _fmt(item, indent + 2, len(pad), len(comma)) + comma)
        return "[\n" + "\n".join(lines) + "\n" + " " * indent + "]"
    return _scalar(value)


def dumps(value):
    """The whole file's text, with a final newline."""
    return _fmt(value, 0, 0, 0) + "\n"
