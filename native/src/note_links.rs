use slint::StyledText;

/// Keep stored notes as plain text; add link markup only for display.
pub fn styled_notes(notes: &str) -> StyledText {
    StyledText::from_markdown(&linkify(notes))
        .unwrap_or_else(|_| StyledText::from_plain_text(notes))
}

fn linkify(notes: &str) -> String {
    let mut out = String::with_capacity(notes.len());
    let mut index = 0;
    while index < notes.len() {
        let rest = &notes[index..];
        if rest.starts_with("https://") || rest.starts_with("http://") {
            let end = rest
                .find(|c: char| {
                    c.is_whitespace()
                        || matches!(
                            c,
                            '<' | '>' | '"' | '\'' | '，' | '。' | '；' | '！' | '？' | '）'
                        )
                })
                .unwrap_or(rest.len());
            let candidate = &rest[..end];
            let url = candidate.trim_end_matches(|c: char| ".,;:!?，。；：！？)]}）".contains(c));
            if url.len() > "https://".len() {
                out.push('<');
                out.push_str(url);
                out.push('>');
                index += url.len();
                continue;
            }
        }
        let ch = rest.chars().next().unwrap();
        match ch {
            '\n' => out.push_str("  \n"),
            '\r' => {}
            '\\' | '*' | '_' | '[' | ']' | '(' | ')' | '`' | '#' | '+' | '-' | '!' | '|' | '~'
            | '>' | '<' | '&' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
        index += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_added_without_reinterpreting_note_text() {
        let note =
            "测评：[入口](https://example.com/a?x=1)。\n备用 https://example.com/b==，密码：a*b";
        let formatted = linkify(note);
        assert!(formatted.contains("<https://example.com/a?x=1>"));
        assert!(formatted.contains("<https://example.com/b==>，"));
        assert!(formatted.contains("密码：a\\*b"));
        assert!(StyledText::from_markdown(&formatted).is_ok());
    }
}
