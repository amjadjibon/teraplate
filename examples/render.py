import pytera
from pathlib import Path

TEMPLATES = str(Path(__file__).parent / "templates/**/*")

engine = pytera.TeraEngine(TEMPLATES)


def render_index():
    html = engine.render("index.html", {
        "page_title": "My Projects",
        "user": {"name": "John"},
        "items": [
            {"name": "pytera",  "badge": "new",  "tags": ["rust", "python"]},
            {"name": "memsh",   "badge": None,   "tags": ["go", "shell"]},
            {"name": "tera",    "badge": None,   "tags": ["rust", "templates"]},
        ],
    })
    print("=== index.html ===")
    print(html)


def render_email():
    txt = engine.render("email.txt", {
        "subject": "Your weekly summary",
        "app_name": "pytera",
        "recipient": {"name": "John", "is_new": False},
        "events": [
            {"date": "2026-04-20", "description": "Published v0.1.0"},
            {"date": "2026-04-21", "description": "Added mixed Python package"},
        ],
    })
    print("=== email.txt ===")
    print(txt)


def render_string():
    out = pytera.render_str(
        "{{ items | length }} item(s): {% for i in items %}{{ i }}{% if not loop.last %}, {% endif %}{% endfor %}",
        {"items": ["Rust", "Python", "Tera"]},
    )
    print("=== pytera.render_str ===")
    print(out)


def render_string_with_engine():
    out = engine.render_str(
        "{% for s in scores %}{{ s.name }}: {{ s.score }}{% if not loop.last %} | {% endif %}{% endfor %}",
        {"scores": [{"name": "Alice", "score": 95}, {"name": "Bob", "score": 87}, {"name": "Carol", "score": 92}]},
    )
    print("=== engine.render_str ===")
    print(out)


if __name__ == "__main__":
    render_index()
    render_email()
    render_string()
    render_string_with_engine()
