"""
pytera — Python bindings for the Tera template engine, powered by Rust and PyO3.

Tera uses Jinja2-compatible syntax: ``{{ variable }}``, ``{% for %}``,
``{% if %}``, template inheritance, filters, and macros.

Quick start::

    import pytera

    # One-off render — no engine needed
    out = pytera.render_once("Hello, {{ name }}!", {"name": "Amjad"})

    # File-based engine
    engine = pytera.TeraEngine("templates/**/*.html")
    html = engine.render("index.html", {"title": "Home"})

    # Inline string render on an existing engine
    html = engine.render_str("<b>{{ msg }}</b>", {"msg": "hi"})
"""

from .pytera import TeraEngine, render_once

__all__ = ["TeraEngine", "render_once"]
