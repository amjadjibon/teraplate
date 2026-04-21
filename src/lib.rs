use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use tera::{Context, Tera};

/// Template engine that loads templates from the filesystem.
///
/// Templates are located via a glob pattern and kept in memory for fast
/// repeated rendering. The syntax is Jinja2-compatible (Tera dialect).
///
/// Args:
///     glob: Glob pattern for template files, e.g. ``"templates/**/*.html"``.
///
/// Raises:
///     ValueError: If the glob pattern is invalid or any matched file cannot
///         be parsed as a Tera template.
///
/// Example::
///
///     engine = TeraEngine("templates/**/*.html")
///     html = engine.render("index.html", {"title": "Home"})
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

    /// Render a named template with the given context.
    ///
    /// The template must have been loaded from disk when the engine was
    /// created (i.e. it matched the glob pattern passed to ``__init__``).
    ///
    /// Args:
    ///     template_name: Path of the template relative to the glob root,
    ///         e.g. ``"pages/index.html"``.
    ///     context: Variable bindings passed to the template. Any
    ///         JSON-serializable Python value is accepted (dicts, lists,
    ///         strings, numbers, booleans, None).
    ///
    /// Returns:
    ///     Rendered output as a string.
    ///
    /// Raises:
    ///     ValueError: If the template is not found or rendering fails.
    ///
    /// Example::
    ///
    ///     html = engine.render("email/welcome.html", {"user": "Amjad"})
    fn render(&self, template_name: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        self.tera
            .render(template_name, &ctx)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Render a raw template string without loading from disk.
    ///
    /// Useful for dynamic or user-supplied templates. The rendered result
    /// is not cached — use :meth:`render` for hot paths.
    ///
    /// Args:
    ///     template_str: A Tera template string, e.g. ``"Hello, {{ name }}!"``.
    ///     context: Variable bindings passed to the template.
    ///
    /// Returns:
    ///     Rendered output as a string.
    ///
    /// Raises:
    ///     ValueError: If the template cannot be parsed or rendering fails.
    ///
    /// Example::
    ///
    ///     out = engine.render_str("{{ x }} + {{ y }} = {{ x + y }}", {"x": 1, "y": 2})
    fn render_str(&self, template_str: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        Tera::one_off(template_str, &ctx, true)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

fn pydict_to_context(dict: &Bound<'_, PyDict>) -> PyResult<Context> {
    let py = dict.py();
    let json_str: String = py.import("json")?.call_method1("dumps", (dict,))?.extract()?;
    let value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Context::from_value(value).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Render a template string in one shot without creating an engine.
///
/// Convenience function for one-off renders. There is no caching — every
/// call parses and renders the template from scratch. For repeated rendering
/// of the same template prefer :class:`TeraEngine`.
///
/// Args:
///     template_str: A Tera template string, e.g. ``"Hello, {{ name }}!"``.
///     context: Variable bindings passed to the template. Any
///         JSON-serializable Python value is accepted.
///
/// Returns:
///     Rendered output as a string.
///
/// Raises:
///     ValueError: If the template cannot be parsed or rendering fails.
///
/// Example::
///
///     out = render_once("Hello, {{ name }}! You have {{ count }} messages.",
///                       {"name": "Amjad", "count": 42})
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
