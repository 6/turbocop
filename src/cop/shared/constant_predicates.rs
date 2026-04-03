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

/// Get the short (demodulized) constant name from a node.
///
/// Matches rubocop-ast's `ConstantNode#short_name`.
///
/// - `Foo`         → `Some(b"Foo")`
/// - `Foo::Bar`    → `Some(b"Bar")`
/// - `::Foo::Bar`  → `Some(b"Bar")`
///
/// Returns `None` if the node is neither a `ConstantReadNode` nor a `ConstantPathNode`.
pub fn constant_short_name<'a>(node: &ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(const_read) = node.as_constant_read_node() {
        Some(const_read.name().as_slice())
    } else if let Some(const_path) = node.as_constant_path_node() {
        // ConstantPathNode always has a name (the rightmost segment).
        // In `Foo::Bar`, name is `Bar`. In `::Foo`, name is `Foo`.
        Some(const_path.name().map_or(b"" as &[u8], |n| n.as_slice()))
    } else {
        None
    }
}

/// Check if a `ConstantPathNode` is absolute (starts with `::`).
///
/// Matches rubocop-ast's `ConstantNode#absolute?`.
///
/// - `::Foo`      → true
/// - `::Foo::Bar` → true
/// - `Foo::Bar`   → false
/// - `Foo`        → N/A (use on ConstantPathNode only)
pub fn is_absolute_constant(node: &ruby_prism::ConstantPathNode<'_>) -> bool {
    // Walk the parent chain to the leftmost segment.
    // If the leftmost ConstantPathNode has parent() == None, it's absolute (cbase).
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
        // Recurse into parent first (left side), then push our name (right side).
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

    fn first_node(code: &str) -> ruby_prism::ParseResult<'_> {
        parse_source(code.as_bytes())
    }

    fn with_first_node<F: Fn(&ruby_prism::Node<'_>)>(code: &str, f: F) {
        let result = first_node(code);
        let program = result.node().as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        f(&node);
    }

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

    #[test]
    fn test_constant_path_segments_simple() {
        with_first_node("Foo", |node| {
            let segs = constant_path_segments(node);
            assert_eq!(segs, vec![b"Foo" as &[u8]]);
        });
    }

    #[test]
    fn test_constant_path_segments_qualified() {
        with_first_node("Foo::Bar::Baz", |node| {
            let segs = constant_path_segments(node);
            assert_eq!(
                segs,
                vec![b"Foo" as &[u8], b"Bar" as &[u8], b"Baz" as &[u8]]
            );
        });
    }

    #[test]
    fn test_constant_path_segments_absolute() {
        with_first_node("::Foo::Bar", |node| {
            let segs = constant_path_segments(node);
            assert_eq!(segs, vec![b"Foo" as &[u8], b"Bar" as &[u8]]);
        });
    }

    #[test]
    fn test_constant_path_segments_non_constant() {
        with_first_node("foo", |node| {
            let segs = constant_path_segments(node);
            assert!(segs.is_empty());
        });
    }
}
