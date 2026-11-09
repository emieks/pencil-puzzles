# Sphinx configuration for the documentation site, served at /docs/ next to
# the web app.
#
#   pip install -r docs/requirements.txt pencil-puzzles
#   sphinx-build -b dirhtml -d target/doctrees docs web/dist/docs

project = "pencil-puzzles"
author = "pencil-puzzles contributors"
copyright = "pencil-puzzles contributors"

extensions = [
    "myst_parser",
    "sphinx.ext.autodoc",
    "sphinx_copybutton",
]

root_doc = "index"
exclude_patterns = [
    "design/**",
    "guidelines.md",
    "genres/TEMPLATE.md",
    "requirements.txt",
]

myst_heading_anchors = 3
suppress_warnings = ["myst.header"]

autodoc_member_order = "bysource"
autodoc_typehints = "signature"
autodoc_preserve_defaults = True
add_module_names = False
default_role = "literal"

copybutton_prompt_text = r">>> |\.\.\. "
copybutton_prompt_is_regexp = True
copybutton_only_copy_prompt_lines = False

html_theme = "furo"
html_title = "pencil-puzzles"
html_static_path = ["_static"]
html_css_files = ["site.css"]
html_copy_source = False
html_show_sourcelink = False
html_theme_options = {
    "source_repository": "https://github.com/emieks/pencil-puzzles/",
    "source_branch": "main",
    "source_directory": "docs/",
    "footer_icons": [
        {
            "name": "GitHub",
            "url": "https://github.com/emieks/pencil-puzzles",
            "html": '<svg stroke="currentColor" fill="currentColor" stroke-width="0" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z"></path></svg>',
            "class": "",
        },
    ],
    "light_css_variables": {
        "font-stack": 'system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
        "font-stack--monospace": 'ui-monospace, "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace',
        "color-brand-primary": "#1d1d1d",
        "color-brand-content": "#1f5aa6",
        "color-brand-visited": "#1f5aa6",
        "color-foreground-primary": "#1d1d1d",
        "color-foreground-secondary": "#555",
        "color-foreground-muted": "#6b6b6b",
        "color-background-primary": "#fff",
        "color-background-secondary": "#f7f7f7",
        "color-background-border": "#e2e2e2",
    },
    "dark_css_variables": {
        "color-brand-primary": "#e2e2e2",
        "color-brand-content": "#8db8f2",
        "color-brand-visited": "#8db8f2",
        "color-foreground-primary": "#e2e2e2",
        "color-foreground-secondary": "#b5b5b5",
        "color-foreground-muted": "#9a9a9a",
        "color-background-primary": "#161616",
        "color-background-secondary": "#1d1d1d",
        "color-background-border": "#333",
    },
}
pygments_style = "default"
pygments_dark_style = "github-dark"


def _hide_puzzle_signature(app, what, name, obj, options, signature, return_annotation):
    """Puzzles are created by the library, so their constructor is not shown."""
    import pencil_puzzles

    if what == "class" and isinstance(obj, type) and issubclass(obj, pencil_puzzles.Puzzle):
        return "", None
    return None


def setup(app):
    app.connect("autodoc-process-signature", _hide_puzzle_signature)
