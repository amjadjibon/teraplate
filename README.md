# teraplate

Python bindings for the [Tera](https://github.com/Keats/tera) template engine, built with Rust, [PyO3](https://github.com/PyO3/pyo3), and [Maturin](https://github.com/PyO3/maturin).

`teraplate` exposes a small Python API over Tera's Jinja2-like template syntax: variables, filters, loops, conditionals, inheritance, and macros.

## Features

- Render named templates loaded from a filesystem glob
- Render inline template strings with `teraplate.render_str(...)`
- Render inline strings on an existing engine with `engine.render_str(...)`
- Inspect loaded template names with `engine.templates()`
- Accept plain Python dictionaries as context
- Support nested JSON-serializable values such as lists, numbers, booleans, and `None`
- Ship type information for Python tooling

## Requirements

- Python `>=3.8`
- Rust toolchain for local source builds

## Installation

Install from PyPI:

```bash
pip install teraplate
```

Install with `uv`:

```bash
uv add teraplate
```

Build from source:

```bash
git clone https://github.com/amjadjibon/tera-py
cd tera-py
uv run maturin develop --release
```

## Quickstart

### Module-level inline render

```python
import teraplate

out = teraplate.render_str(
    "Hello, {{ name }}! You have {{ count }} messages.",
    {"name": "Alex", "count": 42},
)

print(out)
# Hello, Alex! You have 42 messages.
```

### File-based rendering

```python
import teraplate

engine = teraplate.TeraEngine("examples/templates/**/*")
html = engine.render(
    "index.html",
    {
        "page_title": "My Projects",
        "user": {"name": "Alex"},
        "items": [
            {"name": "teraplate", "badge": "new", "tags": ["rust", "python"]},
            {"name": "tera", "badge": None, "tags": ["templates"]},
        ],
    },
)

print(html)
```

### Inline render on an existing engine

```python
import teraplate

engine = teraplate.TeraEngine("examples/templates/**/*")
out = engine.render_str(
    "{% for s in scores %}{{ s.name }}: {{ s.score }}{% if not loop.last %} | {% endif %}{% endfor %}",
    {"scores": [{"name": "Alice", "score": 95}, {"name": "Bob", "score": 87}]},
)

print(out)
# Alice: 95 | Bob: 87
```

## API

### `teraplate.TeraEngine(glob: str)`

Loads templates from a glob pattern.

```python
engine = teraplate.TeraEngine("examples/templates/**/*")
```

Raises `TemplateLoadError` if the glob is invalid or any matched template cannot be parsed.

### `engine.render(template_name: str, context: dict) -> str`

Renders a template that was loaded when the engine was created.

```python
html = engine.render("index.html", {"page_title": "Home"})
```

Raises `TemplateNotFoundError` if the template is missing, and `TemplateRenderError` for other rendering failures.

### `engine.render_str(template_str: str, context: dict) -> str`

Renders a raw template string without reading from disk.

```python
out = engine.render_str("Hello {{ name }}", {"name": "Alex"})
```

### `engine.templates() -> list[str]`

Returns the names of templates currently loaded in the engine.

```python
loaded = sorted(engine.templates())
```

### `teraplate.render_str(template_str: str, context: dict) -> str`

Renders a raw template string without creating an engine.

```python
out = teraplate.render_str("{{ x }} + {{ y }} = {{ x + y }}", {"x": 1, "y": 2})
```

## Exceptions

`teraplate` exports Python exception types so callers can catch specific failures:

- `TeraplateError`: base exception for package-specific errors
- `TemplateLoadError`: templates could not be loaded or parsed from disk
- `TemplateRenderError`: rendering failed
- `TemplateNotFoundError`: a named template was not found
- `ContextError`: the Python context could not be converted into Tera context

## Context Rules

Context values are serialized from Python into JSON before being passed to Tera. In practice:

- The top-level context must be a Python `dict`
- Values should be JSON-serializable
- Nested dicts, lists, strings, numbers, booleans, and `None` are supported

If you pass non-JSON Python objects, rendering will fail.

## Template Syntax

`teraplate` uses Tera syntax, which is close to Jinja2.

### Variables

```html
<h1>Hello, {{ name }}!</h1>
<p>Price: {{ product.price }}</p>
```

### Filters

```html
{{ name | upper }}
{{ description | truncate(length=100) }}
{{ items | length }}
```

### Conditionals

```html
{% if logged_in %}
  <a href="/logout">Logout</a>
{% elif is_guest %}
  <a href="/login">Login</a>
{% else %}
  <a href="/register">Register</a>
{% endif %}
```

### Loops

```html
<ul>
{% for item in items %}
  <li>{{ loop.index }}. {{ item }}</li>
{% endfor %}
</ul>
```

### Template Inheritance

`base.html`

```html
<!DOCTYPE html>
<html>
<head><title>{% block title %}My Site{% endblock title %}</title></head>
<body>
  {% block content %}{% endblock content %}
</body>
</html>
```

`page.html`

```html
{% extends "base.html" %}

{% block title %}About{% endblock title %}

{% block content %}
  <h1>About Us</h1>
{% endblock content %}
```

### Macros

```html
{% macro input(name, type="text", value="") %}
  <input type="{{ type }}" name="{{ name }}" value="{{ value }}">
{% endmacro input %}

{{ self::input(name="username") }}
```

## Examples

The repository includes runnable examples in [`examples/render.py`](./examples/render.py) and templates in [`examples/templates/`](./examples/templates/).

Run them with:

```bash
python examples/render.py
```

## Project Layout

```text
tera-py/
├── Cargo.toml
├── pyproject.toml
├── teraplate/
│   ├── __init__.py
│   ├── py.typed
│   └── teraplate.pyi
├── src/
│   └── lib.rs
└── examples/
    ├── render.py
    └── templates/
```

## Development

Build and run locally:

```bash
uv run maturin develop --release
python examples/render.py
cargo test
```
