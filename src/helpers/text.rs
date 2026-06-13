use ratatui::crossterm::event::KeyModifiers;

// Truncate from the end using "..." while counting characters rather than bytes.
pub fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    if text.chars().count() <= max_width {
        return text.to_string();
    }

    // Very narrow fields cannot fit both content and a full ellipsis.
    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let truncated: String = text.chars().take(max_width - 3).collect();
    format!("{truncated}...")
}

// Truncate from the start so important filename suffixes remain visible.
pub fn truncate_start_with_ellipsis(text: &str, max_width: usize) -> String {
    let len = text.chars().count();
    if len <= max_width {
        return text.to_string();
    }

    // Very narrow fields cannot fit both content and a full ellipsis.
    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let truncated: String = text.chars().skip(len - (max_width - 3)).collect();
    format!("...{truncated}")
}

pub fn empty_state_top_padding(visible_height: usize) -> usize {
    visible_height.saturating_sub(1) / 2
}

// Wrap by character count for long tokens such as hashes or paths.
pub fn wrap_chars(content: String, max_width: usize) -> Vec<String> {
    let mut wrapped_lines = Vec::new();

    for line in content.split('\n') {
        if line.is_empty() {
            // Preserve intentional blank lines in commit messages and file content.
            wrapped_lines.push(String::new());
            continue;
        }

        let char_vec: Vec<char> = line.chars().collect();
        let mut start = 0;

        while start < char_vec.len() {
            let end = (start + max_width).min(char_vec.len());
            let slice: String = char_vec[start..end].iter().collect();
            wrapped_lines.push(slice);
            start = end;
        }
    }

    wrapped_lines
}
// Wrap by words while preserving whitespace and indentation.
pub fn wrap_words(content: String, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![content.to_string()];
    }

    let mut wrapped_lines = Vec::new();

    for line in content.split('\n') {
        if line.is_empty() {
            wrapped_lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        let mut current_width = 0;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            if c.is_whitespace() {
                // Keep whitespace runs together so indentation does not collapse.
                let mut space = String::from(c);
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_whitespace() {
                        space.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Overflowing whitespace starts the next line with the same indentation.
                if current_width + space.chars().count() > max_width {
                    wrapped_lines.push(current_line);
                    current_line = space.clone();
                    current_width = space.chars().count();
                } else {
                    current_line.push_str(&space);
                    current_width += space.chars().count();
                }
            } else {
                // Consume a full word before deciding whether it fits on this line.
                let mut word = String::from(c);
                while let Some(&next_c) = chars.peek() {
                    if !next_c.is_whitespace() {
                        word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Long words fall back to character wrapping so lines never overflow.
                if word.chars().count() > max_width {
                    if !current_line.is_empty() {
                        wrapped_lines.push(current_line);
                        current_line = String::new();
                        current_width = 0;
                    }

                    let char_wrapped = wrap_chars(word.clone(), max_width);
                    wrapped_lines.extend(char_wrapped);
                    continue;
                }

                // A word that does not fit starts a fresh line.
                if current_width + word.chars().count() > max_width {
                    if !current_line.is_empty() {
                        wrapped_lines.push(current_line);
                    }
                    current_line = word.clone();
                    current_width = word.chars().count();
                } else {
                    current_line.push_str(&word);
                    current_width += word.chars().count();
                }
            }
        }

        // Push the line being accumulated after the input line is exhausted.
        wrapped_lines.push(current_line);
    }

    wrapped_lines
}

// Center a line with left padding only, matching terminal text rendering.
pub fn center_line(line: &str, width: usize) -> String {
    if line.chars().count() >= width {
        line.to_string()
    } else {
        let padding = (width - line.chars().count()) / 2;
        format!("{}{}", " ".repeat(padding), line)
    }
}

// Decode blobs from common encodings before the viewer sanitizes control characters.
pub fn decode(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        // UTF-16 little endian with BOM.
        let utf16: Vec<u16> = bytes[2..].chunks(2).map(|c| u16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)])).collect();
        String::from_utf16(&utf16).unwrap_or_default()
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16 big endian with BOM.
        let utf16: Vec<u16> = bytes[2..].chunks(2).map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)])).collect();
        String::from_utf16(&utf16).unwrap_or_default()
    } else if bytes.len() > 1 && bytes[1] == 0 {
        // A zero second byte is a practical signal for UTF-16 little endian text.
        let utf16: Vec<u16> = bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)])).collect();
        String::from_utf16(&utf16).unwrap_or_default()
    } else {
        // Lossy UTF-8 keeps binary-ish content from crashing the viewer.
        String::from_utf8_lossy(bytes).to_string()
    }
}

// Normalize text for terminal display and strip non-newline control characters.
pub fn sanitize(string: String) -> String {
    string
        .replace("\r\n", "\n")
        .replace("\r", "\n") // Normalize old Mac newlines.
        .chars()
        .flat_map(|character| match character {
            '\t' => "    ".chars().collect::<Vec<_>>(),    // Expand tabs to stable columns.
            '\n' => vec!['\n'],                            // Preserve line boundaries.
            character if character.is_control() => vec![], // Drop non-printing controls.
            _ => vec![character],                          // Keep printable content.
        })
        .collect()
}

// Convert crossterm modifiers into the keybinding label format.
pub fn modifiers_to_string(mods: KeyModifiers) -> String {
    let mut parts = Vec::new();
    if mods.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if mods.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }
    if mods.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }
    parts.join(" + ")
}

pub fn pascal_to_spaced(s: &str) -> String {
    let mut result = String::new();

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            result.push(' ');
        }
        result.push(c);
    }

    result
}

pub fn fill_width(left: &str, right: &str, width: usize) -> String {
    let left_len = left.chars().count();
    let right_len = right.chars().count();
    let spaces = width.saturating_sub(left_len + right_len).max(1);
    truncate_with_ellipsis(format!("{}{}{}", left, " ".repeat(spaces), right).as_str(), width)
}
