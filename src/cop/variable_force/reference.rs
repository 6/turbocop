/// A single reference (read) of a variable.
#[derive(Debug)]
pub struct Reference {
    /// Byte offset of the reference node in source.
    pub node_offset: usize,
    /// Index into the scope stack at the time the reference was recorded.
    /// Used to determine if the reference crosses a scope boundary.
    pub scope_index: usize,
    /// Whether this is an explicit reference (e.g., `x` in code) vs an
    /// implicit one (e.g., bare `super` implicitly references all method
    /// args, `binding()` implicitly captures all accessible variables).
    pub explicit: bool,
}

impl Reference {
    pub fn new(node_offset: usize, scope_index: usize) -> Self {
        Self {
            node_offset,
            scope_index,
            explicit: true,
        }
    }

    pub fn implicit(node_offset: usize, scope_index: usize) -> Self {
        Self {
            node_offset,
            scope_index,
            explicit: false,
        }
    }
}
