use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use tera::{Context, Error as TeraError, ErrorKind, Tera};

create_exception!(teraplate, TeraplateError, PyException, "Base exception for teraplate.");
create_exception!(
    teraplate,
    TemplateLoadError,
    TeraplateError,
    "Raised when templates cannot be loaded or parsed from disk."
);
create_exception!(
    teraplate,
    TemplateRenderError,
    TeraplateError,
    "Raised when template rendering fails."
);
create_exception!(
    teraplate,
    TemplateNotFoundError,
    TemplateRenderError,
    "Raised when a named template cannot be found."
);
create_exception!(
    teraplate,
    ContextError,
    TeraplateError,
    "Raised when Python context data cannot be converted into Tera context."
);

fn map_tera_load_error(error: TeraError) -> PyErr {
    TemplateLoadError::new_err(error.to_string())
}

fn map_tera_render_error(error: TeraError) -> PyErr {
    match &error.kind {
        ErrorKind::TemplateNotFound(_) | ErrorKind::MissingParent { .. } => {
            TemplateNotFoundError::new_err(error.to_string())
        }
        _ => TemplateRenderError::new_err(error.to_string()),
    }
}

fn map_context_error(message: impl ToString) -> PyErr {
    ContextError::new_err(message.to_string())
}

/// Template engine that loads templates from the filesystem.
///
/// Templates are located via a glob pattern and kept in memory for fast
/// repeated rendering. The syntax is Jinja2-compatible (Tera dialect).
///
/// Args:
///     glob: Glob pattern for template files, e.g. ``"templates/**/*.html"``.
///
/// Raises:
///     TemplateLoadError: If the glob pattern is invalid or any matched file
///         cannot be parsed as a Tera template.
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
        let tera = Tera::new(glob).map_err(map_tera_load_error)?;
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
    ///     TemplateNotFoundError: If the template is not found.
    ///     TemplateRenderError: If rendering fails for another reason.
    ///
    /// Example::
    ///
    ///     html = engine.render("email/welcome.html", {"user": "Alex"})
    fn render(&self, template_name: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        self.tera
            .render(template_name, &ctx)
            .map_err(map_tera_render_error)
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
    ///     ContextError: If the context cannot be converted.
    ///     TemplateRenderError: If the template cannot be parsed or rendering
    ///         fails.
    ///
    /// Example::
    ///
    ///     out = engine.render_str("{{ x }} + {{ y }} = {{ x + y }}", {"x": 1, "y": 2})
    fn render_str(&self, template_str: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        Tera::one_off(template_str, &ctx, true)
            .map_err(map_tera_render_error)
    }

    /// Return the list of currently loaded template names.
    ///
    /// Returns:
    ///     list[str]: List of loaded template names.
    ///
    /// Example::
    ///
    ///     engine = TeraEngine("templates/**/*.html")
    ///     sorted(engine.templates())
    fn templates(&self) -> Vec<String> {
        self.tera
            .get_template_names()
            .map(str::to_owned)
            .collect()
    }
}

fn pydict_to_context(dict: &Bound<'_, PyDict>) -> PyResult<Context> {
    let py = dict.py();
    let json_str: String = py
        .import("json")
        .map_err(|e| map_context_error(e))?
        .call_method1("dumps", (dict,))
        .map_err(|e| map_context_error(e))?
        .extract()
        .map_err(|e| map_context_error(e))?;
    let value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(map_context_error)?;
    Context::from_value(value).map_err(map_context_error)
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
///     ContextError: If the context cannot be converted.
///     TemplateRenderError: If the template cannot be parsed or rendering
///         fails.
///
/// Example::
///
///     out = render_str("Hello, {{ name }}! You have {{ count }} messages.",
///                      {"name": "Alex", "count": 42})
#[pyfunction]
fn render_str(template_str: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
    let ctx = pydict_to_context(context)?;
    Tera::one_off(template_str, &ctx, true).map_err(map_tera_render_error)
}

#[pymodule]
fn teraplate(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    m.add_class::<TeraEngine>()?;
    m.add_function(wrap_pyfunction!(render_str, m)?)?;
    m.add("TeraplateError", py.get_type::<TeraplateError>())?;
    m.add("TemplateLoadError", py.get_type::<TemplateLoadError>())?;
    m.add("TemplateRenderError", py.get_type::<TemplateRenderError>())?;
    m.add("TemplateNotFoundError", py.get_type::<TemplateNotFoundError>())?;
    m.add("ContextError", py.get_type::<ContextError>())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::IntoPyDict;
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

    fn add_error_types<'py>(py: Python<'py>) -> Bound<'py, PyDict> {
        [
            ("TeraplateError", py.get_type::<TeraplateError>()),
            ("TemplateLoadError", py.get_type::<TemplateLoadError>()),
            ("TemplateRenderError", py.get_type::<TemplateRenderError>()),
            ("TemplateNotFoundError", py.get_type::<TemplateNotFoundError>()),
            ("ContextError", py.get_type::<ContextError>()),
        ]
        .into_py_dict(py)
        .expect("error types dict should be created")
    }

    struct TempTemplateDir {
        path: PathBuf,
    }

    impl TempTemplateDir {
        fn new() -> Self {
            let unique = format!(
                "teraplate-tests-{}-{}",
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
    fn exported_error_types_have_expected_hierarchy() {
        with_python(|py| -> PyResult<()> {
            let ctx = add_error_types(py);

            py.run(
                c"assert issubclass(TemplateLoadError, TeraplateError)\nassert issubclass(TemplateRenderError, TeraplateError)\nassert issubclass(TemplateNotFoundError, TemplateRenderError)\nassert issubclass(ContextError, TeraplateError)",
                None,
                Some(&ctx),
            )?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn new_returns_template_load_error_for_invalid_templates() {
        let templates = TempTemplateDir::new();
        templates.write("broken.html", "{% if user %}");

        with_python(|py| -> PyResult<()> {
            let error = match TeraEngine::new(&templates.glob()) {
                Ok(_) => panic!("expected invalid template to fail loading"),
                Err(error) => error,
            };

            assert!(error.is_instance_of::<TemplateLoadError>(py));
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
            context.set_item("items", vec!["teraplate", "tera"])?;

            let result = engine.render("index.html", &context)?;

            assert!(result.contains("<title>Projects</title>"));
            assert!(result.contains("[teraplate][tera]"));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn templates_returns_loaded_template_names() {
        let templates = TempTemplateDir::new();
        templates.write("base.html", "<title>{% block title %}Default{% endblock title %}</title>");
        templates.write("pages/index.html", "{% extends \"base.html\" %}");

        with_python(|_py| -> PyResult<()> {
            let engine = TeraEngine::new(&templates.glob())?;
            let mut loaded = engine.templates();
            loaded.sort();

            assert_eq!(loaded, vec!["base.html".to_string(), "pages/index.html".to_string()]);
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
    fn render_returns_template_not_found_error_for_missing_template() {
        with_python(|py| -> PyResult<()> {
            let engine = TeraEngine {
                tera: Tera::default(),
            };
            let context = PyDict::new(py);

            let error = engine.render("missing.html", &context).unwrap_err();

            assert!(error.is_instance_of::<TemplateNotFoundError>(py));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn module_render_str_rejects_non_json_values_with_context_error() {
        with_python(|py| -> PyResult<()> {
            let object = py.import("builtins")?.getattr("object")?.call0()?;
            let context = PyDict::new(py);
            context.set_item("value", object)?;

            let error = render_str("{{ value }}", &context).unwrap_err();

            assert!(error.is_instance_of::<ContextError>(py));
            assert!(error.is_instance_of::<TeraplateError>(py));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn render_str_returns_template_render_error_for_invalid_template_source() {
        with_python(|py| -> PyResult<()> {
            let context = PyDict::new(py);

            let error = render_str("{% if user %}", &context).unwrap_err();

            assert!(error.is_instance_of::<TemplateRenderError>(py));
            assert!(error.is_instance_of::<TeraplateError>(py));
            Ok(())
        })
        .unwrap();
    }
}
