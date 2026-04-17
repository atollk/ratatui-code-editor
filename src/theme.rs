use crate::utils;
use ratatui_core::style::{Color, Style};
use std::collections::HashMap;

pub fn vesper() -> HashMap<String, Style> {
    let raw = vec![
        ("identifier", "#A5FCB6"),
        ("field_identifier", "#A5FCB6"),
        ("property_identifier", "#A5FCB6"),
        ("property", "#A5FCB6"),
        ("string", "#b1fce5"),
        ("keyword", "#a0a0a0"),
        ("constant", "#f6c99f"),
        ("number", "#f6c99f"),
        ("integer", "#f6c99f"),
        ("float", "#f6c99f"),
        ("variable", "#ffffff"),
        ("variable.builtin", "#ffffff"),
        ("function", "#f6c99f"),
        ("function.call", "#f6c99f"),
        ("method", "#f6c99f"),
        ("function.macro", "#f6c99f"),
        ("comment", "#585858"),
        ("namespace", "#f6c99f"),
        ("type", "#f6c99f"),
        ("type.builtin", "#f6c99f"),
        ("tag.attribute", "#c6a5fc"),
        ("tag", "#c6a5fc"),
        ("error", "#A5FCB6"),
    ];
    build_theme(&raw)
}

pub fn build_theme(theme: &Vec<(&str, &str)>) -> HashMap<String, Style> {
    theme
        .into_iter()
        .map(|(name, hex)| {
            let (r, g, b) = utils::rgb(hex);
            (name.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
        })
        .collect()
}
