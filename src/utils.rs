pub fn indent(lang_name: Option<&str>) -> &'static str {
    if let Some(lang) = lang_name {
        match lang {
            "rust" | "python" | "php" | "toml" | "c" | "cpp" | "zig" | "kotlin" | "erlang"
            | "html" | "sql" => "    ",
            "go" | "c_sharp" => "\t",
            _ => "  ",
        }
    } else {
        "  "
    }
}

pub fn comment(lang_name: Option<&str>) -> &'static str {
    if let Some(lang) = lang_name {
        match lang {
            "python" | "shell" => "#",
            "lua" => "--",
            _ => "//",
        }
    } else {
        "//"
    }
}

pub fn count_indent_units(
    line: ropey::RopeSlice<'_>,
    indent_unit: &str,
    max_col: Option<usize>,
) -> usize {
    if indent_unit.is_empty() {
        return 0;
    }

    let mut chars = line.chars();
    let mut count = 0;
    let mut col = 0;
    let indent_chars: Vec<char> = indent_unit.chars().collect();

    'outer: loop {
        for &ch in &indent_chars {
            match chars.next() {
                Some(c) if c == ch => col += 1,
                _ => break 'outer,
            }
        }
        count += 1;
        if let Some(max) = max_col {
            if col >= max {
                break;
            }
        }
    }

    count
}

pub fn rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

/// Calculate end position by walking through the text
/// Returns (end_row, end_col) starting from (start_row, start_col)
pub fn calculate_end_position(start_row: usize, start_col: usize, text: &str) -> (usize, usize) {
    let mut end_row = start_row;
    let mut end_col = start_col;

    for ch in text.chars() {
        if ch == '\n' {
            end_row += 1;
            end_col = 0;
        } else {
            end_col += 1;
        }
    }

    (end_row, end_col)
}
