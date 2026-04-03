//! Shared constant node predicates, mirroring rubocop-ast's `ConstantNode` mixin.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/constant_node.rb`
//!
//! Provides helpers for `ConstantReadNode`, `ConstantPathNode`, and related
//! constant reference/assignment nodes.
//!
//! ## Prism vs parser gem differences
//!
//! - Parser gem uses `const` for both `Foo` and `Foo::Bar`. Prism splits these into
//!   `ConstantReadNode` (simple `Foo`) and `ConstantPathNode` (qualified `Foo::Bar`).
//! - `::Foo` in Prism is a `ConstantPathNode` whose `parent()` is `None` (representing cbase).
//! - `casgn` maps to `ConstantWriteNode` (simple) or `ConstantPathWriteNode` (qualified).

use crate::parse::source::SourceFile;

// ---------------------------------------------------------------------------
// Name extraction
// ---------------------------------------------------------------------------

/// Get the last segment name of a constant node.
///
/// Matches rubocop-ast's `ConstantNode#short_name`.
///
/// - `Foo`         → `Some(b"Foo")`
/// - `Foo::Bar`    → `Some(b"Bar")`
/// - `::Foo::Bar`  → `Some(b"Bar")`
/// - `foo`         → `None`
pub fn constant_short_name<'a>(node: &ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(const_read) = node.as_constant_read_node() {
        Some(const_read.name().as_slice())
    } else if let Some(const_path) = node.as_constant_path_node() {
        Some(const_path.name().map_or(b"" as &[u8], |n| n.as_slice()))
    } else {
        None
    }
}

/// Get the full constant path string from source bytes.
///
/// For a ConstantPathNode like `ActiveRecord::Base`, extracts the full text.
pub fn full_constant_path<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> &'a [u8] {
    let loc = node.location();
    &source.as_bytes()[loc.start_offset()..loc.end_offset()]
}

// ---------------------------------------------------------------------------
// Simple constant matching
// ---------------------------------------------------------------------------

/// Check if a node is a simple constant matching `(const {nil? cbase} :Name)`.
///
/// Returns true for `Name` (ConstantReadNode) or `::Name` (ConstantPathNode
/// with cbase parent), but NOT for qualified paths like `Foo::Name`.
pub fn is_simple_constant(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == name;
    }
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(n) = cp.name() {
            if n.as_slice() != name {
                return false;
            }
            // cbase: parent is None (e.g., `::Date`)
            return cp.parent().is_none();
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Path predicates (from rubocop-ast ConstantNode mixin)
// ---------------------------------------------------------------------------

/// Check if a `ConstantPathNode` is absolute (starts with `::`).
///
/// Matches rubocop-ast's `ConstantNode#absolute?`.
///
/// - `::Foo`      → true
/// - `::Foo::Bar` → true
/// - `Foo::Bar`   → false
/// - `Foo`        → N/A (use on ConstantPathNode only)
pub fn is_absolute_constant(node: &ruby_prism::ConstantPathNode<'_>) -> bool {
    let mut current_parent = node.parent();
    loop {
        match current_parent {
            None => return true, // cbase (::)
            Some(ref p) if p.as_constant_path_node().is_some() => {
                current_parent = p.as_constant_path_node().unwrap().parent();
            }
            Some(_) => return false, // some other node (e.g. ConstantReadNode)
        }
    }
}

/// Check if a `ConstantPathNode` is relative (does not start with `::`).
///
/// Matches rubocop-ast's `ConstantNode#relative?`.
pub fn is_relative_constant(node: &ruby_prism::ConstantPathNode<'_>) -> bool {
    !is_absolute_constant(node)
}

/// Collect the name segments of a constant path from left to right.
///
/// Matches rubocop-ast's `ConstantNode#each_path` (yielding names, not nodes).
///
/// - `Foo::Bar::Baz`   → `["Foo", "Bar", "Baz"]`
/// - `::Foo::Bar`      → `["Foo", "Bar"]`
/// - `Foo`             → `["Foo"]` (if passed as a Node, not ConstantPathNode)
///
/// Accepts a generic `Node` so callers don't need to pre-match.
/// Returns an empty vec for non-constant nodes.
pub fn constant_path_segments<'a>(node: &ruby_prism::Node<'a>) -> Vec<&'a [u8]> {
    let mut segments = Vec::new();
    collect_segments(node, &mut segments);
    segments
}

fn collect_segments<'a>(node: &ruby_prism::Node<'a>, out: &mut Vec<&'a [u8]>) {
    if let Some(const_read) = node.as_constant_read_node() {
        out.push(const_read.name().as_slice());
    } else if let Some(const_path) = node.as_constant_path_node() {
        if let Some(parent) = const_path.parent() {
            collect_segments(&parent, out);
        }
        if let Some(name) = const_path.name() {
            out.push(name.as_slice());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_source;

    fn with_first_node<F: Fn(&ruby_prism::Node<'_>)>(code: &str, f: F) {
        let result = parse_source(code.as_bytes());
        let program = result.node().as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        f(&node);
    }

    // ── constant_short_name ────────────────────────────────────────────────────

    #[test]
    fn test_constant_short_name_simple() {
        with_first_node("Foo", |node| {
            assert_eq!(constant_short_name(node), Some(b"Foo" as &[u8]));
        });
    }

    #[test]
    fn test_constant_short_name_qualified() {
        with_first_node("Foo::Bar", |node| {
            assert_eq!(constant_short_name(node), Some(b"Bar" as &[u8]));
        });
    }

    #[test]
    fn test_constant_short_name_absolute() {
        with_first_node("::Foo::Bar", |node| {
            assert_eq!(constant_short_name(node), Some(b"Bar" as &[u8]));
        });
    }

    #[test]
    fn test_constant_short_name_non_constant() {
        with_first_node("foo", |node| {
            assert_eq!(constant_short_name(node), None);
        });
    }

    // ── is_simple_constant ───────────────────────────────────────────────

    #[test]
    fn test_is_simple_constant_read() {
        with_first_node("Foo", |node| {
            assert!(is_simple_constant(node, b"Foo"));
            assert!(!is_simple_constant(node, b"Bar"));
        });
    }

    #[test]
    fn test_is_simple_constant_absolute() {
        with_first_node("::Foo", |node| {
            assert!(is_simple_constant(node, b"Foo"));
        });
    }

    #[test]
    fn test_is_simple_constant_qualified_rejects() {
        with_first_node("Bar::Foo", |node| {
            assert!(!is_simple_constant(node, b"Foo"));
        });
    }

    #[test]
    fn test_is_simple_constant_non_constant() {
        with_first_node("foo", |node| {
            assert!(!is_simple_constant(node, b"foo"));
        });
    }

    // ── is_absolute_constant / is_relative_constant ──────────────────────

    #[test]
    fn test_is_absolute_constant() {
        with_first_node("::Foo", |node| {
            let cp = node.as_constant_path_node().unwrap();
            assert!(is_absolute_constant(&cp));
        });
        with_first_node("::Foo::Bar", |node| {
            let cp = node.as_constant_path_node().unwrap();
            assert!(is_absolute_constant(&cp));
        });
    }

    #[test]
    fn test_is_relative_constant() {
        with_first_node("Foo::Bar", |node| {
            let cp = node.as_constant_path_node().unwrap();
            assert!(is_relative_constant(&cp));
        });
    }

    // ── constant_path_segments ───────────────────────────────────────────

    #[test]
    fn test_constant_path_segments_simple() {
        with_first_node("Foo", |node| {
            assert_eq!(constant_path_segments(node), vec![b"Foo" as &[u8]]);
        });
    }

    #[test]
    fn test_constant_path_segments_qualified() {
        with_first_node("Foo::Bar::Baz", |node| {
            assert_eq!(
                constant_path_segments(node),
                vec![b"Foo" as &[u8], b"Bar" as &[u8], b"Baz" as &[u8]]
            );
        });
    }

    #[test]
    fn test_constant_path_segments_absolute() {
        with_first_node("::Foo::Bar", |node| {
            assert_eq!(
                constant_path_segments(node),
                vec![b"Foo" as &[u8], b"Bar" as &[u8]]
            );
        });
    }

    #[test]
    fn test_constant_path_segments_non_constant() {
        with_first_node("foo", |node| {
            assert!(constant_path_segments(node).is_empty());
        });
    }
}
