"""
teraplate — Python bindings for the Tera template engine, powered by Rust and PyO3.

Tera uses Jinja2-compatible syntax: ``{{ variable }}``, ``{% for %}``,
``{% if %}``, template inheritance, filters, and macros.

Quick start::

    import teraplate

    # Inline render — no engine needed
    out = teraplate.render_str("Hello, {{ name }}!", {"name": "Alex"})

    # File-based engine
    engine = teraplate.TeraEngine("templates/**/*.html")
    html = engine.render("index.html", {"title": "Home"})

    # Inline string render on an existing engine
    html = engine.render_str("<b>{{ msg }}</b>", {"msg": "hi"})
"""

from .teraplate import TeraEngine, render_str
from .teraplate import (
    ContextError,
    TeraplateError,
    TemplateLoadError,
    TemplateNotFoundError,
    TemplateRenderError,
)

__all__ = [
    "ContextError",
    "TeraplateError",
    "TemplateLoadError",
    "TemplateNotFoundError",
    "TemplateRenderError",
    "TeraEngine",
    "render_str",
]
