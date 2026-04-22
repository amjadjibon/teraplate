"""
tera-py — Python bindings for the Tera template engine, powered by Rust and PyO3.

Tera uses Jinja2-compatible syntax: ``{{ variable }}``, ``{% for %}``,
``{% if %}``, template inheritance, filters, and macros.

Quick start::

    import tera_py

    # Inline render — no engine needed
    out = tera_py.render_str("Hello, {{ name }}!", {"name": "Alex"})

    # File-based engine
    engine = tera_py.TeraEngine("templates/**/*.html")
    html = engine.render("index.html", {"title": "Home"})

    # Inline string render on an existing engine
    html = engine.render_str("<b>{{ msg }}</b>", {"msg": "hi"})
"""

from .tera_py import TeraEngine, render_str
from .tera_py import (
    ContextError,
    TeraPyError,
    TemplateLoadError,
    TemplateNotFoundError,
    TemplateRenderError,
)

__all__ = [
    "ContextError",
    "TeraPyError",
    "TemplateLoadError",
    "TemplateNotFoundError",
    "TemplateRenderError",
    "TeraEngine",
    "render_str",
]
