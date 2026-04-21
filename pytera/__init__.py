"""
pytera — Python bindings for the Tera template engine, powered by Rust and PyO3.

Tera uses Jinja2-compatible syntax: ``{{ variable }}``, ``{% for %}``,
``{% if %}``, template inheritance, filters, and macros.

Quick start::

    import pytera

    # Inline render — no engine needed
    out = pytera.render_str("Hello, {{ name }}!", {"name": "Alex"})

    # File-based engine
    engine = pytera.TeraEngine("templates/**/*.html")
    html = engine.render("index.html", {"title": "Home"})

    # Inline string render on an existing engine
    html = engine.render_str("<b>{{ msg }}</b>", {"msg": "hi"})
"""

from .pytera import TeraEngine, render_str

__all__ = ["TeraEngine", "render_str"]
