pub fn clean_response(input: &str, raw_output: bool) -> String {
    if raw_output {
        return input.to_string();
    }

    let mut cleaned = input.trim().to_string();
    cleaned = strip_code_fence(&cleaned).trim().to_string();
    cleaned = strip_opening_preamble(&cleaned);
    cleaned = strip_trailing_meta(&cleaned);
    cleaned.trim().to_string()
}

fn strip_code_fence(input: &str) -> String {
    let trimmed = input.trim();
    if !trimmed.starts_with("```") || !trimmed.ends_with("```") {
        return input.to_string();
    }

    let without_opening = trimmed.trim_start_matches("```");
    let Some((first_line, rest)) = without_opening.split_once('\n') else {
        return input.to_string();
    };

    let body = if first_line.trim().is_empty() || is_language_hint(first_line.trim()) {
        rest
    } else {
        without_opening
    };

    body.trim_end_matches("```").trim().to_string()
}

fn strip_opening_preamble(input: &str) -> String {
    let mut lines: Vec<&str> = input.lines().collect();
    if let Some(first) = lines.first() {
        let first_trimmed = first.trim_start();
        let preamble = ["Sure,", "Here is", "Here's", "Certainly,", "Of course,"];
        if preamble
            .iter()
            .any(|prefix| first_trimmed.starts_with(prefix))
        {
            lines.remove(0);
        }
    }

    lines.join("\n")
}

fn strip_trailing_meta(input: &str) -> String {
    let mut lines: Vec<&str> = input.lines().collect();
    let meta = ["Note:", "Please note", "I have", "The text"];

    while let Some(last) = lines.last() {
        let trimmed = last.trim_start();
        if meta.iter().any(|prefix| trimmed.starts_with(prefix)) {
            lines.pop();
        } else {
            break;
        }
    }

    lines.join("\n")
}

fn is_language_hint(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '+')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markdown_code_fence() {
        let cleaned = clean_response("```text\nHello\n```", false);

        assert_eq!(cleaned, "Hello");
    }

    #[test]
    fn strips_common_preamble_and_trailing_meta() {
        let cleaned = clean_response(
            "Sure, here is the corrected text:\nHello world.\nNote: I fixed grammar.",
            false,
        );

        assert_eq!(cleaned, "Hello world.");
    }

    #[test]
    fn raw_output_bypasses_cleaning() {
        let raw = "  Sure,\nHello  ";

        assert_eq!(clean_response(raw, true), raw);
    }
}
