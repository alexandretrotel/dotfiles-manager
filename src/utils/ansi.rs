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
