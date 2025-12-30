from mkdocs.plugins import BasePlugin
from mkdocs.config import config_options
from pathlib import Path
from dataclasses import dataclass

import kindaxml


@dataclass
class DocExample:
    title: str
    description: str
    input_text: str
    explanation: str
    parser_config_explanation: str = "default"
    parser_config: kindaxml.ParserConfig | None = None

    def get_output(self) -> str:
        """Render the input text using KindaXML and return HTML output."""
        result = kindaxml.parse(self.input_text, self.parser_config)

        # Ensure markers are sorted by absolute position for stable processing
        markers = sorted(result.markers, key=lambda m: m.pos)

        rendered = ""
        seg_start = 0
        m_i = 0  # pointer into markers list

        for segment in result.segments:
            seg_text0 = segment.text
            seg_end = seg_start + len(seg_text0)

            # Build popup text for the segment (unchanged logic)
            popup_text: list[str] = []
            if segment.annotations:
                for ann in segment.annotations:
                    attrs = [f"{k}={v!r}" for k, v in ann.attrs.items()]
                    popup_text.append(f"{ann.tag} [{', '.join(attrs)}]")

            # Collect markers that fall within this segment (in global coords)
            hits: list[tuple[int, str]] = []
            while m_i < len(markers) and markers[m_i].pos < seg_end:
                marker = markers[m_i]
                if marker.pos >= seg_start:
                    idx = marker.pos - seg_start  # segment-local insertion index

                    m_attrs = [f"{k}={v!r}" for k, v in marker.annotation.attrs.items()]
                    ins = (
                        f"{marker.annotation.tag} [{', '.join(m_attrs)}] (empty marker)"
                    )
                    hits.append((idx, DocExample.render_output("_", [ins])))
                m_i += 1

            # Stable insertion with a "pieces" builder (no shifting indices)
            if not hits:
                seg_text = seg_text0
            else:
                # hits are already in ascending idx because markers are sorted,
                # but sort again to be safe if positions can tie or reorder.
                hits.sort(key=lambda x: x[0])

                out_parts: list[str] = []
                cur = 0
                for idx, ins in hits:
                    # Clamp idx just in case (defensive)
                    if idx < cur:
                        idx = cur
                    if idx > len(seg_text0):
                        idx = len(seg_text0)

                    out_parts.append(seg_text0[cur:idx])
                    out_parts.append(ins)
                    cur = idx
                out_parts.append(seg_text0[cur:])
                seg_text = "".join(out_parts)

            rendered += self.render_output(seg_text, popup_text)

            # advance position using ORIGINAL segment text length
            seg_start = seg_end

        return rendered

    @classmethod
    def render_output(cls, text: str, popup_content: list[str]) -> str:
        """Render a text segment with optional popup content."""
        data_pop = ""
        if popup_content:
            data_pop = 'class="hl-pop" data-pop="{}"'.format(
                "&#10;".join(popup_content)
            )
        return f"""<span {data_pop}>{text}</span>"""


examples = [
    DocExample(
        title="Inline span (explicit close)",
        description="Showing how <cite> tags are parsed and annotated.",
        input_text="We shipped <cite id=1>last week</cite>.",
        explanation="the <code>cite</code> tag is recognized and closed normally, so the annotation applies only to the inner span.",
        parser_config=None,
    ),
    DocExample(
        title="Tag annotations",
        description="All tags annotations support with quoted attributes, boolean attributes, and multiple attributes.",
        input_text=(
            "Words can have <tag a=1 b='two' c d=\"4\">multiple attributes</tag>. "
            "Word can also have <tag 59=42 9000>number as attribute</tag>. Attribute "
            "<tag no=quote>without</tag> quotation mark works, and there will be best-effort to auto-close "
            "<tag att='one two three>un-closed quotation marks</tag>. "
            "Unrecognized tags are <unknown foo=bar>auto dropped</unknown> with the default config."
        ),
        explanation="`risk` is configured with `forward_next_token`, so only the next token is annotated for the second tag. Whereas `mytag` uses `retro_line`, so it attaches backward to the start of the line.",
        parser_config_explanation="Tag annotations can have multiple attributes, including boolean attributes (no value), with or without quotes.",
        parser_config=kindaxml.ParserConfig().with_recognized_tags(["tag"]),
    ),
    DocExample(
        title="Retroactive cite (unclosed + auto-close)",
        description="Showing how unclosed <cite> tags are handled.",
        input_text="We shipped last week <cite id=1>. More info <note>soon.",
        explanation="`cite` is configured with `retro_line`, so the unclosed tag attaches backward on the same line up to the tag start. The following `<note>` triggers auto-close behavior, because it reaches the end of the line.",
        parser_config=None,
    ),
    DocExample(
        title="Forward token (per-tag strategy)",
        description="Setting individual tag recovery strategies.",
        input_text="Risks: <mytag level=high> load tests are late. <risk level=low>Docs slipping.",
        explanation="`risk` is configured with `forward_next_token`, so only the next token is annotated for the second tag. Whereas `mytag` uses `retro_line`, so it attaches backward to the start of the line.",
        parser_config_explanation="`risk` recovery strategy set to `forward_next_token`",
        parser_config=kindaxml.ParserConfig()
        .with_recovery_strategy("mytag", "retro_line")
        .with_recovery_strategy("risk", "forward_next_token"),
    ),
    DocExample(
        title="Self-closing markers",
        description="Using self-closing tags will emit zero-width markers.",
        input_text="Todo list: <todo id=7/>finish rollout <todo/> update docs.",
        explanation="Self-closing `<todo/>` tags emit zero-width markers at their positions instead of annotating a span.",
        # parser_config_explanation="self-closing tags emit zero-width markers at their positions instead of annotating a span.",
        parser_config=kindaxml.ParserConfig().with_recognized_tags(["todo"]),
    ),
]


class TransformPlugin(BasePlugin):
    config_scheme = (
        ("out_file", config_options.Type(str, default="generated/snippets.md")),
    )

    def on_pre_build(self, config, **kwargs):
        out_rel = self.config["out_file"]
        out_path = Path(config["docs_dir"]) / out_rel
        out_path.parent.mkdir(parents=True, exist_ok=True)

        output = ""
        for example in examples:
            output += render_example(example)
        out_path.write_text(output, encoding="utf-8")


def render_example(example: DocExample) -> str:
    """Render a DocExample into an HTML snippet with input and output side by side."""
    input_html = Html.p(f"\n```xml\n{example.input_text}\n```\n")
    output_html = Html.p(example.get_output())

    body = example.description
    parser_setting = ""
    parser_setting += f"*{example.parser_config_explanation}*, aka: "
    if example.parser_config is None:
        config = kindaxml.ParserConfig.default_cite_config()
    else:
        config = example.parser_config
    parser_setting += f"\n```python\n{config}\n```\n"
    body += f"\n\n**Parse settings**: {parser_setting}\n\n"
    content_prefix = f"## {example.title}\n" + body
    content_suffix = (
        Html.p("") + "**Technical detail:** " + example.explanation + "\n\n"
    )

    two_col = Html.two_col(
        "Input",
        input_html,
        "Rendered Output",
        output_html,
    )
    return content_prefix + two_col + content_suffix


class Html:
    @classmethod
    def t(cls, tag: str, content: str, class_name: str | None = None) -> str:
        cls_attr = f' class="{class_name}"' if class_name else ""
        return f"<{tag}{cls_attr}>{content}</{tag}>"

    @classmethod
    def p(cls, content: str, class_name: str | None = None) -> str:
        return cls.t("p", content, class_name)

    @classmethod
    def div(cls, content: str, class_name: str) -> str:
        return cls.t("div", content, class_name)

    @classmethod
    def panel(cls, title: str, body: str) -> str:
        return Html.div(cls.t("h4", title) + cls.t("p", body), "panel")

    @classmethod
    def two_col(
        cls,
        left_title: str,
        left_body: str,
        right_title: str,
        right_body: str,
    ) -> str:
        left_panel = cls.panel(left_title, left_body)
        right_panel = cls.panel(right_title, right_body)
        return cls.div(left_panel + right_panel, "two-col")
