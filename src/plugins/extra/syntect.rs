//! Syntax highlighting for code blocks

pub use syntect;

use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use crate::{
    MarkdownIt, Node, NodeValue, Renderer,
    parser::core::CoreRule,
    plugins::cmark::block::{code::CodeBlock, fence::CodeFence},
};

#[derive(Debug)]
pub struct SyntectSnippet {
    pub html: String,
}

impl NodeValue for SyntectSnippet {
    fn render(&self, _: &Node, fmt: &mut dyn Renderer) {
        fmt.text_raw(&self.html);
    }
}

pub fn add(md: &mut MarkdownIt) {
    md.add_rule::<SyntectRule>();
}

pub struct SyntectRule;
impl CoreRule for SyntectRule {
    fn run(root: &mut Node, _: &MarkdownIt) {
        let ss = SyntaxSet::load_defaults_newlines();

        root.walk_mut(|node, _| {
            let (content, language) = if let Some(data) = node.cast::<CodeBlock>() {
                (Some(&data.content), None)
            } else if let Some(data) = node.cast::<CodeFence>() {
                (Some(&data.content), Some(&data.info))
            } else {
                Default::default()
            };

            if let Some(content) = content {
                let syntax = language
                    .and_then(|language| ss.find_syntax_by_token(language))
                    .unwrap_or_else(|| ss.find_syntax_plain_text());

                let mut html_generator =
                    ClassedHTMLGenerator::new_with_class_style(syntax, &ss, ClassStyle::Spaced);

                for line in LinesWithEndings::from(content) {
                    if html_generator
                        .parse_html_for_line_which_includes_newline(line)
                        .is_err()
                    {
                        return;
                    }
                }

                let content = html_generator.finalize();

                if let Some(data) = node.cast_mut::<CodeBlock>() {
                    data.content = content;
                    data.raw = true;
                } else if let Some(data) = node.cast_mut::<CodeFence>() {
                    data.content = content;
                    data.raw = true;
                }

                node.attrs.push(("class".into(), "code".into()));
            }
        });
    }
}
