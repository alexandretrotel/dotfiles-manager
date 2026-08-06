/// Strips ANSI escape sequences (color/cursor codes) from `input`.
pub(crate) fn strip_ansi_codes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.next_if_eq(&'[').is_some() {
            chars.by_ref().find(|c| ('@'..='~').contains(c));
            continue;
        }

        output.push(ch);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_color_codes() {
        assert_eq!(strip_ansi_codes("\u{1b}[31mred\u{1b}[0m"), "red");
    }

    #[test]
    fn leaves_plain_text_unchanged() {
        assert_eq!(strip_ansi_codes("plain text"), "plain text");
    }

    #[test]
    fn strips_multiple_sequences_in_one_string() {
        assert_eq!(
            strip_ansi_codes("\u{1b}[1m\u{1b}[32mbold green\u{1b}[0m normal"),
            "bold green normal"
        );
    }

    #[test]
    fn handles_empty_string() {
        assert_eq!(strip_ansi_codes(""), "");
    }

    #[test]
    fn leaves_lone_escape_without_bracket_intact() {
        assert_eq!(strip_ansi_codes("\u{1b}not a csi"), "\u{1b}not a csi");
    }

    #[test]
    fn strips_cursor_movement_codes() {
        assert_eq!(strip_ansi_codes("\u{1b}[2K\u{1b}[1Ghello"), "hello");
    }
}
