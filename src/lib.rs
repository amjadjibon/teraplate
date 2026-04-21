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
    ///     html = engine.render("email/welcome.html", {"user": "Alex"})
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

/// Render a template string without creating an engine.
///
/// There is no caching — every
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
///     out = render_str("Hello, {{ name }}! You have {{ count }} messages.",
///                      {"name": "Alex", "count": 42})
#[pyfunction]
fn render_str(template_str: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
    let ctx = pydict_to_context(context)?;
    Tera::one_off(template_str, &ctx, true).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pymodule]
fn pytera(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TeraEngine>()?;
    m.add_function(wrap_pyfunction!(render_str, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::exceptions::{PyTypeError, PyValueError};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Once;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn with_python<F, R>(f: F) -> R
    where
        F: for<'py> FnOnce(Python<'py>) -> R,
    {
        static PYTHON: Once = Once::new();

        PYTHON.call_once(Python::initialize);
        Python::attach(f)
    }

    struct TempTemplateDir {
        path: PathBuf,
    }

    impl TempTemplateDir {
        fn new() -> Self {
            let unique = format!(
                "pytera-tests-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("system time should be after unix epoch")
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);

            fs::create_dir_all(&path).expect("temp template directory should be created");

            Self { path }
        }

        fn write(&self, relative_path: &str, contents: &str) {
            let path = self.path.join(relative_path);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("template parent directory should be created");
            }

            fs::write(path, contents).expect("template file should be written");
        }

        fn glob(&self) -> String {
            format!("{}/**/*", self.path.display())
        }
    }

    impl Drop for TempTemplateDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn module_render_str_renders_nested_context() {
        with_python(|py| -> PyResult<()> {
            let user = PyDict::new(py);
            user.set_item("name", "Alex")?;

            let context = PyDict::new(py);
            context.set_item("user", user)?;
            context.set_item("count", 3)?;

            let result = render_str(
                "Hello, {{ user.name }}! You have {{ count }} messages.",
                &context,
            )?;

            assert_eq!(result, "Hello, Alex! You have 3 messages.");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn new_and_render_load_templates_from_disk() {
        let templates = TempTemplateDir::new();
        templates.write(
            "base.html",
            "<title>{% block title %}Default{% endblock title %}</title>{% block content %}{% endblock content %}",
        );
        templates.write(
            "index.html",
            "{% extends \"base.html\" %}{% block title %}{{ page_title }}{% endblock title %}{% block content %}{% for item in items %}[{{ item }}]{% endfor %}{% endblock content %}",
        );

        with_python(|py| -> PyResult<()> {
            let engine = TeraEngine::new(&templates.glob())?;
            let context = PyDict::new(py);
            context.set_item("page_title", "Projects")?;
            context.set_item("items", vec!["pytera", "tera"])?;

            let result = engine.render("index.html", &context)?;

            assert!(result.contains("<title>Projects</title>"));
            assert!(result.contains("[pytera][tera]"));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn render_str_renders_without_disk_templates() {
        with_python(|py| -> PyResult<()> {
            let engine = TeraEngine {
                tera: Tera::default(),
            };
            let context = PyDict::new(py);
            context.set_item("values", vec![1, 2, 3])?;

            let result = engine.render_str(
                "{% for value in values %}{{ value }}{% if not loop.last %}, {% endif %}{% endfor %}",
                &context,
            )?;

            assert_eq!(result, "1, 2, 3");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn render_returns_value_error_for_missing_template() {
        with_python(|py| -> PyResult<()> {
            let engine = TeraEngine {
                tera: Tera::default(),
            };
            let context = PyDict::new(py);

            let error = engine.render("missing.html", &context).unwrap_err();

            assert!(error.is_instance_of::<PyValueError>(py));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn module_render_str_rejects_non_json_values() {
        with_python(|py| -> PyResult<()> {
            let object = py.import("builtins")?.getattr("object")?.call0()?;
            let context = PyDict::new(py);
            context.set_item("value", object)?;

            let error = render_str("{{ value }}", &context).unwrap_err();

            assert!(error.is_instance_of::<PyTypeError>(py));
            Ok(())
        })
        .unwrap();
    }
}
