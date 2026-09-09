// apps/engine/src/core/markdown.rs

use ffm_parser::{ParserOptions, create_fuyeor_markdown_parser, to_plain_text};

/// Convert Markdown/FFM into visible text for full-text indexing.
pub fn to_search_text(markdown: &str) -> String {
    let parser = create_fuyeor_markdown_parser(ParserOptions::default());
    let ast = parser.parse(markdown);
    to_plain_text(&ast)
}

#[cfg(test)]
mod tests {
    use super::to_search_text;

    #[test]
    fn removes_markdown_presentation_noise_but_keeps_visible_text() {
        assert_eq!(
            to_search_text("# Search\n\nUse **SPP** and [FON](https://example.com)."),
            "Search\nUse SPP and FON."
        );
    }

    #[test]
    fn treats_html_as_text_and_keeps_code_content() {
        assert_eq!(
            to_search_text("<b>HTML</b>\n\n```rust\nlet query = \"search\";\n```"),
            "<b>HTML</b>\nlet query = \"search\";"
        );
    }
}
