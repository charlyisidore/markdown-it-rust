//! Add identifiers, classes and attributes with the syntax `{#id .class key=value}`.

use crate::{
    MarkdownIt, Node,
    parser::{core::CoreRule, inline::Text},
    plugins::cmark::block::{fence::CodeFence, heading::ATXHeading, lheading::SetextHeader},
};

/// Add identifiers, classes and attributes with the syntax `{#id .class key=value}`.
pub fn add(md: &mut MarkdownIt) {
    md.add_rule::<AttrsRule>();
}

struct AttrsRule;

impl CoreRule for AttrsRule {
    fn run(root: &mut Node, _: &MarkdownIt) {
        root.walk_mut(|node, _| {
            if node.is::<ATXHeading>() || node.is::<SetextHeader>() {
                // # Header {#foo}
                let Some(text) = node
                    .children
                    .last_mut()
                    .and_then(|child| child.cast_mut::<Text>())
                else {
                    return;
                };

                let (content, attrs) = parse_attrs(&text.content);

                if attrs.is_empty() {
                    return;
                }

                text.content = content.to_string();
                node.attrs.extend(attrs);
            } else if let Some(code_fence) = node.cast_mut::<CodeFence>() {
                // ```rust {#foo}
                // println!("Hello world");
                // ```
                let (info, attrs) = parse_attrs(&code_fence.info);

                if attrs.is_empty() {
                    return;
                }

                code_fence.info = info.to_string();
                node.attrs.extend(attrs);
            }
        });
    }
}

/// Parse attributes including the curly braces.
fn parse_attrs(s: &str) -> (&str, Vec<(String, String)>) {
    enum State {
        Start,
        Blank,
        Key,
        Equal,
        Quoted,
        Unquoted,
    }

    let fail = (s, Vec::new());

    let mut attrs = Vec::new();

    let mut state = State::Start;
    let mut key = String::new();
    let mut value = String::new();
    let end;

    // Parse backwards from the end
    let mut char_indices = s.char_indices().rev();

    loop {
        let index_char = char_indices.next();

        state = match state {
            State::Start => match index_char {
                // {#foo}
                //      ^
                Some((_, '}')) => State::Blank,
                _ => return fail,
            },
            State::Blank => match index_char {
                Some((i, c)) => match c {
                    // { key="val" }
                    // ^
                    '{' => {
                        end = i;
                        break;
                    }
                    // { key="val" }
                    //           ^
                    '"' => {
                        value = String::new();
                        State::Quoted
                    }
                    // { key="val" }
                    //            ^
                    c if c.is_ascii_whitespace() => State::Blank,
                    // { key=val }
                    //         ^
                    c => {
                        value = String::new();
                        value.insert(0, c);
                        State::Unquoted
                    }
                },
                // ^key="val" }
                // ^
                None => return fail,
            },
            State::Quoted => match index_char {
                Some((_, c)) => match c {
                    // { key="val" }
                    //       ^
                    '"' => State::Equal,
                    // { key="val" }
                    //          ^
                    c => {
                        value.insert(0, c);
                        State::Quoted
                    }
                },
                // ^val" }
                // ^
                None => return fail,
            },
            State::Equal => match index_char {
                Some((_, c)) => match c {
                    // { key="va\"l" }
                    //          ^
                    '\\' => {
                        value.insert(0, '"');
                        State::Quoted
                    }
                    // { key="val" }
                    //      ^
                    '=' => {
                        key = String::new();
                        State::Key
                    }
                    // { "val" }
                    //  ^
                    _ => return fail,
                },
                // ^"val" }
                // ^
                _ => return fail,
            },
            State::Unquoted => match index_char {
                Some((_, c)) => match c {
                    // {val}
                    // ^
                    '{' => return fail,
                    // {#id}
                    //  ^
                    '#' => {
                        attrs.insert(0, ("id".to_string(), value.clone()));
                        State::Blank
                    }
                    // {.class}
                    //  ^
                    '.' => {
                        attrs.insert(0, ("class".to_string(), value.clone()));
                        State::Blank
                    }
                    // {key=val}
                    //     ^
                    '=' => {
                        key = String::new();
                        State::Key
                    }
                    // { val }
                    //  ^
                    c if c.is_ascii_whitespace() => return fail,
                    // { key=val }
                    //        ^
                    c => {
                        value.insert(0, c);
                        State::Unquoted
                    }
                },
                // ^val }
                // ^
                None => return fail,
            },
            State::Key => match index_char {
                Some((i, c)) => match c {
                    // {key=val}
                    // ^
                    // { key=val }
                    //  ^
                    c if c == '{' || c.is_ascii_whitespace() => {
                        attrs.insert(0, (key.clone(), value.clone()));
                        if c == '{' {
                            end = i;
                            break;
                        }
                        State::Blank
                    }
                    // { key=val }
                    //    ^
                    c => {
                        key.insert(0, c);
                        State::Key
                    }
                },
                // ^key=val }
                // ^
                None => return fail,
            },
        };

        debug_assert!(index_char.is_some());
    }

    if attrs.is_empty() {
        return fail;
    }

    (s[..end].trim_end(), attrs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> String {
        let md = &mut crate::MarkdownIt::new();
        crate::plugins::cmark::add(md);
        super::add(md);
        md.parse(src).render()
    }

    #[test]
    fn parse_attrs_id() {
        assert_eq!(
            parse_attrs("{#foo}"),
            ("", vec![("id".into(), "foo".into())])
        );
    }

    #[test]
    fn parse_attrs_class() {
        assert_eq!(
            parse_attrs("{.haskell}"),
            ("", vec![("class".into(), "haskell".into())])
        );
    }

    #[test]
    fn parse_attrs_key_value() {
        assert_eq!(
            parse_attrs("{key=val}"),
            ("", vec![("key".into(), "val".into())])
        );
    }

    #[test]
    fn parse_attrs_key_value_quoted() {
        assert_eq!(
            parse_attrs(r#"{key2="val 2"}"#),
            ("", vec![("key2".into(), "val 2".into())]),
        );
        assert_eq!(
            parse_attrs(r#"{key2="val\"2"}"#),
            ("", vec![("key2".into(), r#"val"2"#.into())]),
        );
    }

    #[test]
    fn parse_attrs_fail() {
        assert_eq!(parse_attrs("{#foo"), ("{#foo", vec![]));
        assert_eq!(parse_attrs("{}"), ("{}", vec![]));
        assert_eq!(parse_attrs("#foo}"), ("#foo}", vec![]));
        assert_eq!(parse_attrs(r#"val" #foo}"#), (r#"val" #foo}"#, vec![]));
        assert_eq!(parse_attrs(r#""val" #foo}"#), (r#""val" #foo}"#, vec![]));
        assert_eq!(parse_attrs("{val #foo}"), ("{val #foo}", vec![]));
        assert_eq!(parse_attrs("{ val #foo}"), ("{ val #foo}", vec![]));
        assert_eq!(parse_attrs("key=val #foo}"), ("key=val #foo}", vec![]));
    }

    #[test]
    fn parse_attrs_multiple() {
        assert_eq!(
            parse_attrs(r#"{#mycode .haskell .numberLines startFrom="100"}"#),
            (
                "",
                vec![
                    ("id".into(), "mycode".into()),
                    ("class".into(), "haskell".into()),
                    ("class".into(), "numberLines".into()),
                    ("startFrom".into(), "100".into()),
                ],
            ),
        );

        assert_eq!(
            parse_attrs(r#"{#id .class key=val key2="val 2"}"#),
            (
                "",
                vec![
                    ("id".into(), "id".into()),
                    ("class".into(), "class".into()),
                    ("key".into(), "val".into()),
                    ("key2".into(), "val 2".into()),
                ],
            ),
        );
    }

    #[test]
    fn heading_attrs() {
        assert_eq!(
            run("# My heading {#foo}"),
            "<h1 id=\"foo\">My heading</h1>\n"
        );
        assert_eq!(
            run("## My heading ##    {#foo}"),
            "<h2 id=\"foo\">My heading ##</h2>\n"
        );
        assert_eq!(
            run("My heading   {#foo}\n---------------"),
            "<h2 id=\"foo\">My heading</h2>\n"
        );
    }

    #[test]
    fn fenced_code_attrs() {
        assert_eq!(
            run(r#"``` {.foo}
bar
```"#),
            "<pre><code class=\"foo\">bar\n</code></pre>\n"
        );
        assert_eq!(
            run(r#"```pascal {.foo}
bar
```"#),
            "<pre><code class=\"foo language-pascal\">bar\n</code></pre>\n"
        );
    }

    #[test]
    fn heading_anchors_attrs() {
        use crate::plugins::extra::heading_anchors;
        let md = &mut crate::MarkdownIt::new();
        crate::plugins::cmark::add(md);
        super::add(md);
        heading_anchors::add(md, heading_anchors::simple_slugify_fn);
        assert_eq!(
            md.parse("# My heading {#foo}").render(),
            "<h1 id=\"foo\">My heading</h1>\n"
        );
    }

    #[cfg(feature = "syntect")]
    #[test]
    fn syntect_attrs() {
        let md = &mut crate::MarkdownIt::new();
        crate::plugins::cmark::add(md);
        super::add(md);
        crate::plugins::extra::syntect::add(md);
        assert_eq!(
            md.parse(
                r#"``` {#foo}
bar
```"#
            )
            .render(),
            "<pre><code id=\"foo\" class=\"code\"><span class=\"text plain\">bar\n</span></code></pre>\n"
        );
    }
}
