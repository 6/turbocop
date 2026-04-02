/// A single assignment to a variable.
///
/// Tracks whether this specific assignment's value is referenced before being
/// reassigned, enabling per-assignment liveness analysis (as opposed to
/// per-variable analysis which can't distinguish dead intermediate assignments).
#[derive(Debug)]
pub struct Assignment {
    /// Byte offset of the assignment node in source.
    pub node_offset: usize,
    /// Whether any reference consumes this assignment's value.
    pub referenced: bool,
    /// Whether this assignment was overwritten by a later assignment before
    /// being referenced.
    pub reassigned: bool,
    /// Byte offsets of nodes that reference this assignment's value.
    pub references: Vec<usize>,
    /// What kind of assignment this is.
    pub kind: AssignmentKind,
}

impl Assignment {
    pub fn new(node_offset: usize, kind: AssignmentKind) -> Self {
        Self {
            node_offset,
            referenced: false,
            reassigned: false,
            references: Vec::new(),
            kind,
        }
    }

    /// Mark this assignment as referenced by a node at the given offset.
    pub fn reference(&mut self, ref_offset: usize) {
        self.referenced = true;
        self.references.push(ref_offset);
    }

    /// Mark this assignment as reassigned (a later assignment overwrites it).
    /// Only marks as reassigned if not yet referenced — if already referenced,
    /// the value was consumed and the reassignment doesn't make it "dead."
    pub fn reassign(&mut self) {
        if !self.referenced {
            self.reassigned = true;
        }
    }

    /// Whether this assignment's value is used (referenced or captured by block).
    /// The `captured_by_block` parameter comes from the parent Variable.
    pub fn used(&self, captured_by_block: bool) -> bool {
        self.referenced || captured_by_block
    }

    /// Whether this is an operator assignment (`+=`, `-=`, etc.) which reads
    /// the variable before writing.
    pub fn is_operator(&self) -> bool {
        matches!(
            self.kind,
            AssignmentKind::Operator | AssignmentKind::LogicalOr | AssignmentKind::LogicalAnd
        )
    }
}

/// The kind of assignment to a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentKind {
    /// `x = expr`
    Simple,
    /// `x += expr`, `x -= expr`, `x *= expr`, etc.
    Operator,
    /// `x ||= expr`
    LogicalOr,
    /// `x &&= expr`
    LogicalAnd,
    /// `a, b = expr` (part of a multi-write)
    Multiple,
    /// `*a = expr` (rest/splat in multi-write)
    Rest,
    /// `for x in collection` — the loop index variable.
    For,
    /// `/(?<x>\w+)/ =~ str` — named capture group.
    RegexpCapture,
    /// `rescue => x` — exception variable.
    ExceptionCapture,
}
