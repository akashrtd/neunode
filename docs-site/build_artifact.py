"""Assemble the 16-page MkDocs site into one self-contained HTML page for Artifact hosting."""
import re, markdown
from pathlib import Path

DOCS = Path(__file__).parent / "docs"

# (file, section title, diataxis type) -- eyebrow encodes the real doc type, not decoration
NAV = [
    ("index.md",                     "Start here",                      "orientation"),
    ("guide/index.md",               "Why Neunode exists",              "tutorial"),
    ("guide/setup.md",               "1. Set up your machine",          "tutorial"),
    ("guide/first-agent.md",         "2. Your first agent",             "tutorial"),
    ("guide/daemon.md",              "3. The daemon and the API",       "tutorial"),
    ("guide/economy.md",             "4. The economy",                  "tutorial"),
    ("guide/mesh.md",                "5. Go multi-node",                "tutorial"),
    ("traps.md",                     "Traps",                           "how-to"),
    ("reference/index.md",           "Reference",                       "reference"),
    ("reference/cli.md",             "CLI reference",                   "reference"),
    ("reference/http-api.md",        "HTTP API reference",              "reference"),
    ("reference/crates.md",          "Crate map",                       "reference"),
    ("explanation/index.md",         "Explanation",                     "explanation"),
    ("explanation/architecture.md",  "How it fits together",            "explanation"),
    ("explanation/http-native.md",   "Why the daemon owns the database","explanation"),
    ("meta.md",                      "About these docs",                "explanation"),
]

def anchor(path): return "s-" + path.replace(".md", "").replace("/", "-").replace("index", "i")

ANCHORS = {p: anchor(p) for p, _, _ in NAV}

def strip_frontmatter(t):
    return re.sub(r"\A---\n.*?\n---\n", "", t, flags=re.S)

def clean(t):
    # Material icon shortcodes have no renderer outside mkdocs-material
    t = re.sub(r":material-[a-z0-9-]+:\{[^}]*\}", "", t)
    t = re.sub(r":material-[a-z0-9-]+:", "", t)
    return t

def rewrite_links(t, src):
    """Point inter-page .md links at in-page anchors."""
    def sub(m):
        label, href = m.group(1), m.group(2)
        if href.startswith(("http://", "https://", "#")):
            return m.group(0)
        base, _, frag = href.partition("#")
        base = base.split("?")[0]
        if not base.endswith(".md"):
            return m.group(0)
        parts = (Path(src).parent / base).parts
        norm = str(Path(*parts)).replace("\\", "/")
        norm = re.sub(r"^(\.\./)+", "", norm)
        while "/../" in norm:
            norm = re.sub(r"[^/]+/\.\./", "", norm)
        target = ANCHORS.get(norm)
        return f"[{label}](#{target})" if target else f"[{label}](#{ANCHORS.get('index.md')})"
    return re.sub(r"\[([^\]]+)\]\(([^)]+)\)", sub, t)

def render(path):
    raw = strip_frontmatter((DOCS / path).read_text())
    raw = rewrite_links(clean(raw), path)
    raw = re.sub(r"^#\s+.*$", "", raw, count=1, flags=re.M)   # section <h2> supplies the title
    md = markdown.Markdown(extensions=[
        "admonition", "tables", "attr_list", "def_list", "md_in_html", "footnotes",
        "pymdownx.details", "pymdownx.superfences", "pymdownx.highlight",
        "pymdownx.inlinehilite", "pymdownx.tabbed", "pymdownx.keys",
        "pymdownx.mark", "pymdownx.caret", "pymdownx.tilde",
    ], extension_configs={
        "pymdownx.highlight": {"pygments_lang_class": True},
        "pymdownx.tabbed": {"alternate_style": True},
    })
    html = md.convert(raw)
    # demote heading levels so the section title is the only h2
    for a, b in ((5, 6), (4, 5), (3, 4), (2, 3)):
        html = html.replace(f"<h{a}", f"<h{b}").replace(f"</h{a}>", f"</h{b}>")
    return html

groups, seen = [], None
for path, title, kind in NAV:
    top = path.split("/")[0] if "/" in path else "root"
    if top != seen:
        groups.append((top, []))
        seen = top
    groups[-1][1].append((path, title, kind))

GROUP_LABEL = {"root": "", "guide": "The guide", "reference": "Reference", "explanation": "Explanation"}

nav_html, body_html = [], []
for top, items in groups:
    label = GROUP_LABEL.get(top, top)
    if label:
        nav_html.append(f'<li class="nav-group">{label}</li>')
    for path, title, kind in items:
        nav_html.append(f'<li><a href="#{ANCHORS[path]}" data-target="{ANCHORS[path]}">{title}</a></li>')
        body_html.append(
            f'<section id="{ANCHORS[path]}" class="doc-section">'
            f'<p class="eyebrow" data-kind="{kind}">{kind}</p>'
            f'<h2>{title}</h2>{render(path)}</section>'
        )

Path(__file__).parent.joinpath("_nav.html").write_text("\n".join(nav_html))
Path(__file__).parent.joinpath("_body.html").write_text("\n".join(body_html))
print(f"sections: {len(NAV)}  nav bytes: {len(''.join(nav_html))}  body bytes: {len(''.join(body_html))}")
