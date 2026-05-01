//! Function name shortening utilities for ROI (Region of Interest) display
//!
//! This module provides functionality to intelligently compact long Rust function names
//! while keeping them identifiable. It handles:
//! * Generic type collapsing (e.g., `<A, B<C, D<E>>>` → `<…>`)
//! * Module path elision keeping first and last segments
//! * Center truncation as last resort

/// Default maximum length for compacted symbol names
pub const DEFAULT_MAX_LEN: usize = 60;

/// Compacts a Rust function/symbol name for readable display.
///
/// # Strategy
/// 1. If the name fits within `max_len` → return as-is.
/// 2. Collapse **inner** nested generic parameters (`<A<B>>` → `<A<…>>`).
/// 3. Elide intermediate path segments keeping first and last
///    (`std::io::default_write_fmt::Adapter` → `std::..::Adapter`).
/// 4. If still too long, collapse remaining top-level generics entirely.
/// 5. If still exceeds limit, apply center truncation with `…`.
pub fn compact_symbol(name: &str, max_len: usize) -> String {
    if char_count(name) <= max_len {
        return name.to_string();
    }

    // Step 1 – collapse inner generics (depth >= 2), keep outer type names
    let collapsed = collapse_inner_generics(name);
    if char_count(&collapsed) <= max_len {
        return collapsed;
    }

    // Step 2 – elide intermediate path segments inside generics and outside
    let shortened = elide_path_segments(&collapsed, max_len);
    if char_count(&shortened) <= max_len {
        return shortened;
    }

    // Step 3 – collapse remaining top-level generics entirely: <...> → <…>
    let fully_collapsed = collapse_top_generics(&shortened);
    if char_count(&fully_collapsed) <= max_len {
        return fully_collapsed;
    }

    // Step 4 – elide paths again after full generic collapse
    let shortened2 = elide_path_segments(&fully_collapsed, max_len);
    if char_count(&shortened2) <= max_len {
        return shortened2;
    }

    // Step 5 – center truncation
    center_truncate(&shortened2, max_len)
}

/// Convenience wrapper using the default maximum length.
pub fn shorten_name(name: &str) -> String {
    compact_symbol(name, DEFAULT_MAX_LEN)
}

// ---------------------------------------------------------------------------
// Helper – char-based length
// ---------------------------------------------------------------------------

/// Returns the number of Unicode characters (not bytes).
fn char_count(s: &str) -> usize {
    s.chars().count()
}

// ---------------------------------------------------------------------------
// Step 1 – collapse inner generics
// ---------------------------------------------------------------------------

/// Collapses only **inner** generics (depth >= 2) while keeping the top-level
/// generic structure intact.
///
/// `<Adapter<StdoutLock> as Write>` → `<Adapter<…> as Write>`
fn collapse_inner_generics(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut depth: usize = 0;
    let mut suppressing = false;

    for ch in s.chars() {
        match ch {
            '<' => {
                depth += 1;
                if depth <= 1 {
                    result.push(ch);
                } else if depth == 2 {
                    result.push_str("<…>");
                    suppressing = true;
                }
            }
            '>' => {
                if depth == 2 {
                    suppressing = false;
                } else if depth == 1 {
                    result.push(ch);
                }
                depth = depth.saturating_sub(1);
            }
            _ => {
                if !suppressing {
                    result.push(ch);
                }
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Step 2 – path segment elision
// ---------------------------------------------------------------------------

/// Elides intermediate path segments, keeping the first and last segments
/// of each qualified name.
///
/// Works both on top-level paths and inside `<...>` blocks.
///
/// `std::io::default_write_fmt::Adapter` → `std::..::Adapter`
fn elide_path_segments(s: &str, max_len: usize) -> String {
    // First, shorten qualified names inside generic blocks
    let with_short_generics = elide_inside_generics(s);
    if char_count(&with_short_generics) <= max_len {
        return with_short_generics;
    }

    // Then shorten the outer path segments
    let segments = split_path(&with_short_generics);
    if segments.len() <= 3 {
        return with_short_generics;
    }

    // Iteratively remove middle segments until we fit
    elide_outer_segments(&segments, max_len)
}

/// Iteratively elides the longest middle segments of the outer path.
fn elide_outer_segments(segments: &[String], max_len: usize) -> String {
    let mut parts = segments.to_vec();

    loop {
        let candidate = parts.join("::");
        if char_count(&candidate) <= max_len {
            return candidate;
        }

        // Priority 1: collapse any standalone generic segment to <…>
        let mut collapsed = false;
        for seg in parts.iter_mut() {
            if seg.starts_with('<') && seg.ends_with('>') && *seg != "<…>" {
                *seg = "<…>".to_string();
                collapsed = true;
                break;
            }
        }
        if collapsed {
            continue;
        }

        if parts.len() <= 3 {
            break;
        }

        // Priority 2: replace the longest middle segment with ".."
        let mut best_idx = None;
        let mut best_len = 0;
        for (i, part) in parts.iter().enumerate().take(parts.len() - 1).skip(1) {
            if part == ".." {
                continue;
            }
            let seg_len = char_count(part);
            if seg_len > best_len {
                best_len = seg_len;
                best_idx = Some(i);
            }
        }

        if let Some(idx) = best_idx {
            // If there's already an adjacent "..", just remove this segment
            let prev_is_dots = idx > 0 && parts[idx - 1] == "..";
            let next_is_dots = idx + 1 < parts.len() && parts[idx + 1] == "..";

            if prev_is_dots || next_is_dots {
                parts.remove(idx);
            } else {
                parts[idx] = "..".to_string();
            }
        } else {
            break;
        }
    }

    parts.join("::")
}

/// Applies path elision inside `<...>` blocks.
///
/// `<std::io::default_write_fmt::Adapter<…> as core::fmt::Write>`
/// → `<std::..::Adapter<…> as core::..::Write>`
fn elide_inside_generics(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut depth: usize = 0;
    let mut current_block = String::new();

    for ch in s.chars() {
        match ch {
            '<' => {
                depth += 1;
                if depth == 1 {
                    result.push_str(&current_block);
                    current_block.clear();
                    result.push('<');
                } else {
                    current_block.push(ch);
                }
            }
            '>' => {
                if depth == 1 {
                    let inner_shortened = elide_generic_inner(&current_block);
                    result.push_str(&inner_shortened);
                    result.push('>');
                    current_block.clear();
                } else {
                    current_block.push(ch);
                }
                depth = depth.saturating_sub(1);
            }
            _ => {
                current_block.push(ch);
            }
        }
    }
    result.push_str(&current_block);
    result
}

/// Elides paths inside a generic block content (between `<` and `>`).
/// Splits by " as " to handle trait impls, then elides each part.
fn elide_generic_inner(inner: &str) -> String {
    if let Some(as_pos) = inner.find(" as ") {
        let type_part = &inner[..as_pos];
        let trait_part = &inner[as_pos + 4..];
        format!("{} as {}", elide_qualified_name(type_part), elide_qualified_name(trait_part))
    } else {
        elide_qualified_name(inner)
    }
}

/// Elides intermediate segments from a qualified name, keeping the first
/// and last segments intact.
///
/// `std::io::default_write_fmt::Adapter` → `std::..::Adapter`
///
/// Handles reference prefixes like `&mut` properly.
fn elide_qualified_name(name: &str) -> String {
    let (prefix, rest) = if let Some(stripped) = name.strip_prefix("&mut ") {
        ("&mut ", stripped)
    } else if let Some(stripped) = name.strip_prefix('&') {
        ("&", stripped)
    } else {
        ("", name)
    };

    let parts: Vec<&str> = rest.split("::").collect();
    if parts.len() <= 3 {
        return name.to_string();
    }

    // Keep first and last, replace middle with ".."
    format!("{}{}::..::{}", prefix, parts[0], parts[parts.len() - 1])
}

// ---------------------------------------------------------------------------
// Step 3 – collapse top-level generics
// ---------------------------------------------------------------------------

/// Collapses entire top-level generics: `<anything>` → `<…>`
fn collapse_top_generics(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut depth: usize = 0;

    for ch in s.chars() {
        match ch {
            '<' => {
                depth += 1;
                if depth == 1 {
                    result.push_str("<…>");
                }
            }
            '>' => {
                depth = depth.saturating_sub(1);
            }
            _ => {
                if depth == 0 {
                    result.push(ch);
                }
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Common utilities
// ---------------------------------------------------------------------------

/// Splits a symbol name by '::' respecting '<>' depth.
fn split_path(s: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut depth: usize = 0;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '<' => {
                depth += 1;
                current.push(ch);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ':' if depth == 0 && i + 1 < chars.len() && chars[i + 1] == ':' => {
                segments.push(current.clone());
                current.clear();
                i += 2;
                continue;
            }
            _ => {
                current.push(ch);
            }
        }
        i += 1;
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

/// Center truncation: `abcdefghij` with max 7 → `abc…hij`
fn center_truncate(s: &str, max_len: usize) -> String {
    if char_count(s) <= max_len {
        return s.to_string();
    }
    let available = max_len.saturating_sub(1); // 1 char for '…'
    let left = available / 2;
    let right = available - left;

    let left_end = s.char_indices().nth(left).map(|(i, _)| i).unwrap_or(s.len());
    let right_start = s.char_indices().rev().nth(right - 1).map(|(i, _)| i).unwrap_or(0);

    format!("{}…{}", &s[..left_end], &s[right_start..])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SYMBOLS: &[&str] = &[
        "_zisk_main",
        "main",
        "guest::main",
        "sha2::sha256::compress256",
        "std::io::stdio::_print",
        "core::fmt::write",
        "<[u8; 32] as core::fmt::Debug>::fmt",
        "<core::fmt::builders::DebugSet>::entry",
        "<std::io::default_write_fmt::Adapter<std::io::stdio::StdoutLock> as core::fmt::Write>::write_str",
        "<&u8 as core::fmt::Debug>::fmt",
        "<u8 as core::fmt::LowerHex>::fmt",
        "<std::io::buffered::linewritershim::LineWriterShim<std::io::stdio::StdoutRaw> as std::io::Write>::write_all",
        "<core::fmt::Formatter>::pad_integral",
        "memcpy",
        "memset",
        "core::slice::memchr::memrchr",
        "ziskos::io::commit::<guest::Output>",
        "<guest::Output as serde_core::ser::Serialize>::serialize::<&mut bincode::ser::Serializer<&mut alloc::vec::Vec<u8>, bincode::config::WithOtherLimit<bincode::config::WithOtherTrailing<bincode::config::WithOtherIntEncoding<bincode::config::DefaultOptions, bincode::config::int::FixintEncoding>, bincode::config::trailing::AllowTrailing>, bincode::config::limit::Infinite>>>",
        "<core::fmt::Formatter>::pad_integral::write_prefix",
        "<std::io::buffered::bufwriter::BufWriter<std::io::stdio::StdoutRaw>>::flush_buf",
        "sys_write",
        "<&mut bincode::ser::Serializer<&mut alloc::vec::Vec<u8>, bincode::config::WithOtherLimit<bincode::config::WithOtherTrailing<bincode::config::WithOtherIntEncoding<bincode::config::DefaultOptions, bincode::config::int::FixintEncoding>, bincode::config::trailing::AllowTrailing>, bincode::config::limit::Infinite>> as serde_core::ser::Serializer>::serialize_u8",
        "ziskos::io::write",
        "<u32 as core::fmt::Display>::fmt",
        "<core::fmt::Formatter>::debug_list",
    ];

    #[test]
    fn all_fit_within_default_limit() {
        for sym in SYMBOLS {
            let result = compact_symbol(sym, DEFAULT_MAX_LEN);
            assert!(
                char_count(&result) <= DEFAULT_MAX_LEN + 2,
                "Too long ({} chars): {result}",
                char_count(&result)
            );
        }
    }

    #[test]
    fn all_fit_within_custom_limit() {
        for max in [40, 50, 60, 80, 100] {
            for sym in SYMBOLS {
                let result = compact_symbol(sym, max);
                assert!(
                    char_count(&result) <= max + 2,
                    "max={max}, too long ({} chars): {result}",
                    char_count(&result)
                );
            }
        }
    }

    #[test]
    fn short_symbols_unchanged() {
        assert_eq!(compact_symbol("main", 60), "main");
        assert_eq!(compact_symbol("memcpy", 60), "memcpy");
        assert_eq!(compact_symbol("_zisk_main", 60), "_zisk_main");
    }

    #[test]
    fn test_shorten_name_wrapper() {
        assert_eq!(shorten_name("main"), "main");
    }

    #[test]
    fn test_custom_max_len() {
        let name = "<core::fmt::builders::DebugSet>::entry";
        // With large max, unchanged
        assert_eq!(compact_symbol(name, 100), name);
        // With small max, gets compacted
        let compacted = compact_symbol(name, 30);
        assert!(char_count(&compacted) <= 32);
    }

    #[test]
    fn test_inner_generic_collapsing() {
        let input = "<Adapter<StdoutLock> as Write>";
        assert_eq!(collapse_inner_generics(input), "<Adapter<…> as Write>");

        let input2 = "<SimpleType>";
        assert_eq!(collapse_inner_generics(input2), "<SimpleType>");
    }

    #[test]
    fn test_top_generic_collapsing() {
        let input = "<Adapter<…> as Write>";
        assert_eq!(collapse_top_generics(input), "<…>");
    }

    #[test]
    fn test_elide_qualified_name() {
        assert_eq!(elide_qualified_name("std::io::default_write_fmt::Adapter"), "std::..::Adapter");
        // Short paths stay as-is
        assert_eq!(elide_qualified_name("core::fmt::Write"), "core::fmt::Write");
        // Handles references
        assert_eq!(
            elide_qualified_name("&mut bincode::ser::foo::Serializer"),
            "&mut bincode::..::Serializer"
        );
    }

    #[test]
    fn test_keeps_outer_types() {
        let input = "<std::io::default_write_fmt::Adapter<std::io::stdio::StdoutLock> as core::fmt::Write>::write_str";
        let result = collapse_inner_generics(input);
        assert!(result.contains("Adapter<…>"), "Got: {}", result);
        assert!(result.contains("Write>::write_str"), "Got: {}", result);
    }

    #[test]
    fn test_path_splitting() {
        let segments = split_path("std::io::stdio::_print");
        assert_eq!(segments, vec!["std", "io", "stdio", "_print"]);

        let segments2 = split_path("Vec<T>::new");
        assert_eq!(segments2, vec!["Vec<T>", "new"]);
    }

    #[test]
    fn test_center_truncate() {
        let input = "abcdefghij";
        let result = center_truncate(input, 7);
        assert_eq!(result.chars().count(), 7);
        assert!(result.contains('…'));
    }

    #[test]
    fn test_all_no_panic() {
        for name in SYMBOLS {
            let shortened = shorten_name(name);
            assert!(!shortened.is_empty(), "Empty result for: {}", name);
        }
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --nocapture
    fn print_all_examples() {
        for max in [60, 80] {
            println!(
                "\n{:>4}  {:<width$}  ORIGINAL",
                "#",
                format!("COMPACTED (max {})", max),
                width = max + 2,
            );
            println!("{}", "═".repeat(max + 120));
            for (i, sym) in SYMBOLS.iter().enumerate() {
                let compact = compact_symbol(sym, max);
                println!("{:>4}  {:<width$}  {}", i + 1, compact, sym, width = max + 2,);
            }
        }
        println!();
    }
}
