class TeraplateError(Exception): ...


class TemplateLoadError(TeraplateError): ...


class TemplateRenderError(TeraplateError): ...


class TemplateNotFoundError(TemplateRenderError): ...


class ContextError(TeraplateError): ...


class TeraEngine:
    """Template engine that loads templates from the filesystem.

    Templates are located via a glob pattern and kept in memory for fast
    repeated rendering.  The syntax is Jinja2-compatible (Tera dialect).

    Args:
        glob: Glob pattern for template files, e.g. ``"templates/**/*.html"``.
            Tera walks the pattern at construction time and loads every match.

    Raises:
        TemplateLoadError: If the glob pattern is invalid or any matched file
            cannot be parsed as a Tera template.

    Example::

        engine = TeraEngine("templates/**/*.html")
        html = engine.render("pages/index.html", {"title": "Home"})
    """

    def __new__(cls, glob: str) -> "TeraEngine": ...

    def render(self, template_name: str, context: dict) -> str:
        """Render a named template with the given context.

        The template must have been loaded from disk when the engine was
        created (i.e. it matched the glob pattern passed to ``__init__``).

        Args:
            template_name: Path of the template relative to the glob root,
                e.g. ``"pages/index.html"``.
            context: Variable bindings passed to the template.  Any
                JSON-serializable Python value is accepted (dicts, lists,
                tuples, strings, numbers, booleans, ``None``).  Dict keys may
                be strings, integers, floats, booleans, or ``None``.

        Returns:
            Rendered output as a string.

        Raises:
            TemplateNotFoundError: If the template is not found.
            TemplateRenderError: If rendering fails for another reason.

        Example::

            html = engine.render("email/welcome.html", {"user": "Alex"})
        """
        ...

    def render_str(self, template_str: str, context: dict) -> str:
        """Render a raw template string without loading from disk.

        Useful for dynamic or user-supplied templates.  Compiled templates are
        cached inside the engine in a bounded LRU cache, so repeated calls with
        the same template skip recompilation without unbounded memory growth.

        Args:
            template_str: A Tera template string, e.g. ``"Hello, {{ name }}!"``.
            context: Variable bindings passed to the template.

        Returns:
            Rendered output as a string.

        Raises:
            ContextError: If the context cannot be converted.
            TemplateRenderError: If the template cannot be parsed or rendering
                fails.

        Example::

            out = engine.render_str("{{ x }} + {{ y }} = {{ x + y }}", {"x": 1, "y": 2})
        """
        ...

    def templates(self) -> list[str]:
        """Return the list of currently loaded template names.

        Returns:
            ``list[str]``: List of currently loaded template names.

        Example::

            engine = TeraEngine("templates/**/*.html")
            sorted(engine.templates())
        """
        ...


def render_str(template_str: str, context: dict) -> str:
    """Render a template string without creating an engine.

    There is no caching — every call parses and renders the template from
    scratch.  For repeated rendering
    of the same template prefer :class:`TeraEngine`.

    Args:
        template_str: A Tera template string, e.g. ``"Hello, {{ name }}!"``.
        context: Variable bindings passed to the template.  Any
            JSON-serializable Python value is accepted. Dict keys may be
            strings, integers, floats, booleans, or ``None``.

    Returns:
        Rendered output as a string.

    Raises:
        ContextError: If the context cannot be converted.
        TemplateRenderError: If the template cannot be parsed or rendering
            fails.

    Example::

        out = render_str(
            "Hello, {{ name }}! You have {{ count }} messages.",
            {"name": "Alex", "count": 42}
        )
    """
    ...
