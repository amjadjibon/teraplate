use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use tera::{Context, Error as TeraError, ErrorKind, Tera};

const INLINE_TEMPLATE_NAME: &str = "__inline__";
const INLINE_TEMPLATE_CACHE_CAPACITY: usize = 128;

create_exception!(
    teraplate,
    TeraplateError,
    PyException,
    "Base exception for teraplate."
);
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

fn map_lock_py_error<T>(e: std::sync::PoisonError<T>) -> PyErr {
    TemplateRenderError::new_err(format!("engine lock poisoned: {}", e))
}

fn map_lock_error<T>(e: std::sync::PoisonError<T>) -> TeraError {
    TeraError::msg(format!("engine lock poisoned: {}", e))
}

struct InlineTemplateCache {
    entries: HashMap<String, Arc<Tera>>,
    usage_order: VecDeque<String>,
    capacity: usize,
}

impl InlineTemplateCache {
    fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            usage_order: VecDeque::new(),
            capacity,
        }
    }

    fn get_or_insert(&mut self, template_str: &str) -> Result<Arc<Tera>, TeraError> {
        if let Some(tera) = self.entries.get(template_str).cloned() {
            self.mark_used(template_str);
            return Ok(tera);
        }

        let mut tera = Tera::default();
        tera.add_raw_template(INLINE_TEMPLATE_NAME, template_str)?;
        let tera = Arc::new(tera);

        if self.capacity == 0 {
            return Ok(tera);
        }

        if self.entries.len() >= self.capacity {
            if let Some(oldest) = self.usage_order.pop_front() {
                self.entries.remove(&oldest);
            }
        }

        let cache_key = template_str.to_owned();
        self.usage_order.push_back(cache_key.clone());
        self.entries.insert(cache_key, Arc::clone(&tera));

        Ok(tera)
    }

    fn mark_used(&mut self, template_str: &str) {
        if let Some(position) = self
            .usage_order
            .iter()
            .position(|cached| cached == template_str)
        {
            self.usage_order.remove(position);
        }
        self.usage_order.push_back(template_str.to_owned());
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

// Direct PyO3 object walk — avoids the json.dumps roundtrip.
// bool must be checked before i64 since Python bools are ints.
fn pyobj_to_key(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = obj.extract::<String>() {
        Ok(s)
    } else if obj.is_none() {
        Ok("null".to_owned())
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(if b { "true" } else { "false" }.to_owned())
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(i.to_string())
    } else if let Ok(u) = obj.extract::<u64>() {
        Ok(u.to_string())
    } else if let Ok(f) = obj.extract::<f64>() {
        if f.is_finite() {
            serde_json::to_string(&f).map_err(map_context_error)
        } else {
            Err(map_context_error("non-finite float"))
        }
    } else {
        Err(map_context_error(format!(
            "unsupported dict key type: {}",
            obj.get_type().name()?
        )))
    }
}

fn pyobj_to_value(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(serde_json::Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(serde_json::Value::Number(i.into()))
    } else if let Ok(u) = obj.extract::<u64>() {
        Ok(serde_json::Value::Number(u.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(serde_json::Value::Number(
            serde_json::Number::from_f64(f).ok_or_else(|| map_context_error("non-finite float"))?,
        ))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(serde_json::Value::String(s))
    } else if let Ok(list) = obj.cast::<PyList>() {
        let arr = list
            .iter()
            .map(|v| pyobj_to_value(&v))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(serde_json::Value::Array(arr))
    } else if let Ok(tuple) = obj.cast::<PyTuple>() {
        let arr = tuple
            .iter()
            .map(|v| pyobj_to_value(&v))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(serde_json::Value::Array(arr))
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            map.insert(pyobj_to_key(&k)?, pyobj_to_value(&v)?);
        }
        Ok(serde_json::Value::Object(map))
    } else {
        Err(map_context_error(format!(
            "unsupported type: {}",
            obj.get_type().name()?
        )))
    }
}

fn pydict_to_context(dict: &Bound<'_, PyDict>) -> PyResult<Context> {
    let value = pyobj_to_value(dict.as_any())?;
    Context::from_value(value).map_err(map_context_error)
}

/// Template engine that loads templates from the filesystem.
///
/// Templates are located via a glob pattern and kept in memory for fast
/// repeated rendering. The syntax is Jinja2-compatible (Tera dialect).
/// The engine is safe to share across threads.
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
    tera: Arc<RwLock<Tera>>,
    inline_templates: Arc<Mutex<InlineTemplateCache>>,
}

#[pymethods]
impl TeraEngine {
    #[new]
    fn new(glob: &str) -> PyResult<Self> {
        let tera = Tera::new(glob).map_err(map_tera_load_error)?;
        Ok(TeraEngine {
            tera: Arc::new(RwLock::new(tera)),
            inline_templates: Arc::new(Mutex::new(InlineTemplateCache::new(
                INLINE_TEMPLATE_CACHE_CAPACITY,
            ))),
        })
    }

    /// Render a named template with the given context.
    ///
    /// The GIL is released during rendering so other Python threads are not blocked.
    ///
    /// Args:
    ///     template_name: Path of the template relative to the glob root,
    ///         e.g. ``"pages/index.html"``.
    ///     context: Variable bindings passed to the template. Any
    ///         JSON-serializable Python value is accepted (dicts, lists,
    ///         tuples, strings, numbers, booleans, None). Dict keys may be
    ///         strings, integers, floats, booleans, or None.
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
    fn render(
        &self,
        py: Python<'_>,
        template_name: &str,
        context: &Bound<'_, PyDict>,
    ) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        let tera = Arc::clone(&self.tera);
        let template_name = template_name.to_owned();

        py.detach(move || {
            tera.read()
                .map_err(map_lock_error)?
                .render(&template_name, &ctx)
        })
        .map_err(map_tera_render_error)
    }

    /// Render a raw template string without loading from disk.
    ///
    /// Compiled templates are cached inside the engine in a bounded LRU cache,
    /// so repeated calls with the same template skip recompilation without
    /// unbounded memory growth.
    /// The GIL is released during rendering.
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
    fn render_str(
        &self,
        py: Python<'_>,
        template_str: &str,
        context: &Bound<'_, PyDict>,
    ) -> PyResult<String> {
        let ctx = pydict_to_context(context)?;
        let inline_templates = Arc::clone(&self.inline_templates);
        let template_str = template_str.to_owned();

        py.detach(move || {
            let tera = inline_templates
                .lock()
                .map_err(map_lock_error)?
                .get_or_insert(&template_str)?;

            tera.render(INLINE_TEMPLATE_NAME, &ctx)
        })
        .map_err(map_tera_render_error)
    }

    /// Return the list of currently loaded template names.
    ///
    /// Returns:
    ///     list[str]: List of loaded template names, excluding cached inline templates.
    ///
    /// Example::
    ///
    ///     engine = TeraEngine("templates/**/*.html")
    ///     sorted(engine.templates())
    fn templates(&self) -> PyResult<Vec<String>> {
        Ok(self
            .tera
            .read()
            .map_err(map_lock_py_error)?
            .get_template_names()
            .filter(|n| !n.starts_with("__inline__"))
            .map(str::to_owned)
            .collect())
    }
}

/// Render a template string without creating an engine.
///
/// There is no caching — every call parses and renders the template from
/// scratch. For repeated rendering of the same template prefer :class:`TeraEngine`.
///
/// Args:
///     template_str: A Tera template string, e.g. ``"Hello, {{ name }}!"``.
///     context: Variable bindings passed to the template. Any
///         JSON-serializable Python value is accepted. Dict keys may be
///         strings, integers, floats, booleans, or None.
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
fn render_str(py: Python<'_>, template_str: &str, context: &Bound<'_, PyDict>) -> PyResult<String> {
    let ctx = pydict_to_context(context)?;
    let template_str = template_str.to_owned();

    py.detach(move || Tera::one_off(&template_str, &ctx, true))
        .map_err(map_tera_render_error)
}

#[pymodule]
fn teraplate(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    m.add_class::<TeraEngine>()?;
    m.add_function(wrap_pyfunction!(render_str, m)?)?;
    m.add("TeraplateError", py.get_type::<TeraplateError>())?;
    m.add("TemplateLoadError", py.get_type::<TemplateLoadError>())?;
    m.add("TemplateRenderError", py.get_type::<TemplateRenderError>())?;
    m.add(
        "TemplateNotFoundError",
        py.get_type::<TemplateNotFoundError>(),
    )?;
    m.add("ContextError", py.get_type::<ContextError>())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::IntoPyDict;
    use serde_json::json;
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
            (
                "TemplateNotFoundError",
                py.get_type::<TemplateNotFoundError>(),
            ),
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
                py,
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

            let result = engine.render(py, "index.html", &context)?;

            assert!(result.contains("<title>Projects</title>"));
            assert!(result.contains("[teraplate][tera]"));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn templates_returns_loaded_template_names() {
        let templates = TempTemplateDir::new();
        templates.write(
            "base.html",
            "<title>{% block title %}Default{% endblock title %}</title>",
        );
        templates.write("pages/index.html", "{% extends \"base.html\" %}");

        with_python(|_py| -> PyResult<()> {
            let engine = TeraEngine::new(&templates.glob())?;
            let mut loaded = engine.templates()?;
            loaded.sort();

            assert_eq!(
                loaded,
                vec!["base.html".to_string(), "pages/index.html".to_string()]
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn render_str_renders_without_disk_templates() {
        with_python(|py| -> PyResult<()> {
            let engine = TeraEngine {
                tera: Arc::new(RwLock::new(Tera::default())),
                inline_templates: Arc::new(Mutex::new(InlineTemplateCache::new(
                    INLINE_TEMPLATE_CACHE_CAPACITY,
                ))),
            };
            let context = PyDict::new(py);
            context.set_item("values", vec![1, 2, 3])?;

            let result = engine.render_str(
                py,
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
                tera: Arc::new(RwLock::new(Tera::default())),
                inline_templates: Arc::new(Mutex::new(InlineTemplateCache::new(
                    INLINE_TEMPLATE_CACHE_CAPACITY,
                ))),
            };
            let context = PyDict::new(py);

            let error = engine.render(py, "missing.html", &context).unwrap_err();

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

            let error = render_str(py, "{{ value }}", &context).unwrap_err();

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

            let error = render_str(py, "{% if user %}", &context).unwrap_err();

            assert!(error.is_instance_of::<TemplateRenderError>(py));
            assert!(error.is_instance_of::<TeraplateError>(py));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn render_str_caches_compiled_template_on_engine() {
        with_python(|py| -> PyResult<()> {
            let engine = TeraEngine {
                tera: Arc::new(RwLock::new(Tera::default())),
                inline_templates: Arc::new(Mutex::new(InlineTemplateCache::new(
                    INLINE_TEMPLATE_CACHE_CAPACITY,
                ))),
            };
            let context = PyDict::new(py);
            context.set_item("name", "Alex")?;
            let tmpl = "Hello, {{ name }}!";

            let first = engine.render_str(py, tmpl, &context)?;
            let second = engine.render_str(py, tmpl, &context)?;

            assert_eq!(first, "Hello, Alex!");
            assert_eq!(second, "Hello, Alex!");
            assert_eq!(
                engine
                    .inline_templates
                    .lock()
                    .expect("inline cache lock should not be poisoned")
                    .len(),
                1
            );

            // inline templates should not appear in templates()
            assert!(engine.templates()?.is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn pydict_to_context_supports_tuples_and_scalar_dict_keys() {
        with_python(|py| -> PyResult<()> {
            let mapping = PyDict::new(py);
            mapping.set_item(2, "two")?;
            mapping.set_item(1.0, "one-point-zero")?;
            mapping.set_item(false, "false-value")?;
            mapping.set_item(py.None(), "null-value")?;

            let context = PyDict::new(py);
            context.set_item("coords", (1, 2, 3))?;
            context.set_item("mapping", mapping)?;

            let ctx = pydict_to_context(&context)?;

            assert_eq!(
                ctx.into_json(),
                json!({
                    "coords": [1, 2, 3],
                    "mapping": {
                        "2": "two",
                        "1.0": "one-point-zero",
                        "false": "false-value",
                        "null": "null-value"
                    }
                })
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn pydict_to_context_rejects_unsupported_dict_key_types() {
        with_python(|py| -> PyResult<()> {
            let object = py.import("builtins")?.getattr("object")?.call0()?;
            let mapping = PyDict::new(py);
            mapping.set_item(object, "value")?;

            let context = PyDict::new(py);
            context.set_item("mapping", mapping)?;

            let error = pydict_to_context(&context).unwrap_err();

            assert!(error.is_instance_of::<ContextError>(py));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn render_str_uses_bounded_lru_cache() {
        with_python(|py| -> PyResult<()> {
            let engine = TeraEngine {
                tera: Arc::new(RwLock::new(Tera::default())),
                inline_templates: Arc::new(Mutex::new(InlineTemplateCache::new(2))),
            };
            let context = PyDict::new(py);
            context.set_item("name", "Alex")?;

            assert_eq!(engine.render_str(py, "A {{ name }}", &context)?, "A Alex");
            assert_eq!(engine.render_str(py, "B {{ name }}", &context)?, "B Alex");
            assert_eq!(engine.render_str(py, "A {{ name }}", &context)?, "A Alex");
            assert_eq!(engine.render_str(py, "C {{ name }}", &context)?, "C Alex");

            let cache = engine
                .inline_templates
                .lock()
                .expect("inline cache lock should not be poisoned");
            assert_eq!(cache.len(), 2);
            assert!(cache.entries.contains_key("A {{ name }}"));
            assert!(cache.entries.contains_key("C {{ name }}"));
            assert!(!cache.entries.contains_key("B {{ name }}"));
            Ok(())
        })
        .unwrap();
    }
}
