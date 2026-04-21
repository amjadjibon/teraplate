# pytera

Python bindings for the [Tera](https://github.com/Keats/tera) template engine, powered by Rust and PyO3.

Tera is a blazing-fast template engine inspired by Jinja2 and Django templates. `pytera` wraps it with a clean Python API via Maturin, giving you nanosecond-level rendering with familiar `{{ variable }}` syntax.

---

## Features

- Full Tera template syntax — variables, filters, loops, conditionals, inheritance, macros
- File-based rendering via glob patterns
- One-off string rendering without a file
- Built on [PyO3](https://github.com/PyO3/pyo3) + [Maturin](https://github.com/PyO3/maturin) — no FFI boilerplate
- Accepts plain Python dicts as template context

---

## Installation

```bash
pip install pytera
```

To build from source (requires Rust):

```bash
git clone https://github.com/yourname/pytera
cd pytera
pip install maturin
maturin develop --release
```

---

## Quickstart

### One-off string rendering

```python
import pytera

result = pytera.render_once(
    "Hello, {{ name }}! You have {{ count }} messages.",
    {"name": "Amjad", "count": 42}
)
print(result)
# Hello, Amjad! You have 42 messages.
```

### File-based rendering

```python
import pytera

engine = pytera.TeraEngine("templates/**/*.html")
html = engine.render("hello.html", {
    "name": "Amjad",
    "items": ["Kubernetes", "Go", "Tera"]
})
print(html)
```

---

## Template Syntax

`pytera` uses Tera's full template syntax, which is nearly identical to Jinja2.

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

`base.html`:
```html
<!DOCTYPE html>
<html>
<head><title>{% block title %}My Site{% endblock title %}</title></head>
<body>
  {% block content %}{% endblock content %}
</body>
</html>
```

`page.html`:
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

---

## API Reference

### `pytera.TeraEngine(glob: str)`

Creates a new engine instance that loads templates from a glob pattern.

```python
engine = pytera.TeraEngine("templates/**/*.html")
```

### `engine.render(template_name: str, context: dict) -> str`

Renders a named template with the given context dict.

```python
html = engine.render("email/welcome.html", {"user": "Amjad"})
```

### `engine.render_str(template_str: str, context: dict) -> str`

Renders a raw template string without loading from disk.

```python
out = engine.render_str("Hello {{ name }}", {"name": "Amjad"})
```

### `pytera.render_once(template_str: str, context: dict) -> str`

Module-level convenience function for one-off rendering. No engine instantiation needed.

```python
out = pytera.render_once("{{ x }} + {{ y }} = {{ x + y }}", {"x": 1, "y": 2})
```

---

## Project Structure

```
pytera/
├── Cargo.toml          # Rust deps: pyo3, tera, serde_json
├── pyproject.toml      # Maturin build config
├── src/
│   └── lib.rs          # PyO3 bindings
└── templates/          # Your Jinja2-compatible templates
    └── hello.html
```

### `Cargo.toml`

```toml
[package]
name = "pytera"
version = "0.1.0"
edition = "2021"

[lib]
name = "pytera"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.22", features = ["extension-module"] }
tera = "1"
serde_json = "1"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

### `pyproject.toml`

```toml
[build-system]
requires = ["maturin>=1.7,<2.0"]
build-backend = "maturin"

[project]
name = "pytera"
version = "0.1.0"
description = "Python bindings for the Tera template engine"
requires-python = ">=3.8"

[tool.maturin]
features = ["pyo3/extension-module"]
```

### `src/lib.rs`

```rust
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use tera::{Context, Tera};

#[pyclass]
struct TeraEngine {
    tera: Tera,
}

#[pymethods]
impl TeraEngine {
    #[new]
    fn new(glob: &str) -> PyResult<Self> {
        let tera = Tera::new(glob).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(TeraEngine { tera })
    }

    fn render(&self, template_name: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        self.tera
            .render(template_name, &ctx)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn render_str(&mut self, template_str: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        Tera::one_off(template_str, &ctx, true)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

fn pydict_to_context(dict: &Bound<'_, PyDict>) -> PyResult<Context> {
    let json_str: String = Python::with_gil(|py| {
        let json_mod = py.import("json")?;
        json_mod.call_method1("dumps", (dict,))?.extract::<String>()
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Context::from_value(value).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
fn render_once(template_str: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
    let ctx = pydict_to_context(context)?;
    Tera::one_off(template_str, &ctx, true).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pymodule]
fn pytera(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TeraEngine>()?;
    m.add_function(wrap_pyfunction!(render_once, m)?)?;
    Ok(())
}
```

---

## Building & Publishing

```bash
# Development install
maturin develop

# Release build
maturin develop --release

# Build wheel
maturin build --release

# Publish to PyPI
maturin publish
```

---

## Why pytera over Jinja2?

| | pytera | Jinja2 |
|---|---|---|
| Runtime | Rust (nanoseconds) | Python (microseconds) |
| Syntax | Tera (Jinja2-compatible) | Jinja2 |
| GIL | Releases during render | Held |
| Template inheritance | ✅ | ✅ |
| Custom filters | Via Rust | Via Python |
| Pure Python fallback | ❌ | ✅ |

For most web apps, Jinja2 is perfectly fast. `pytera` is the right choice when you need **maximum rendering throughput** — bulk email generation, static site generation, high-QPS API responses with templated output.

---

## License

MIT