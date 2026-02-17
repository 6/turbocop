a ? (b ? b1 : b2) : a2
     ^ Style/NestedTernaryOperator: Ternary operators must not be nested. Prefer `if` or `else` constructs instead.

a ? b : c ? d : e
        ^^^^^^^^^ Style/NestedTernaryOperator: Ternary operators must not be nested. Prefer `if` or `else` constructs instead.

x ? (y ? 1 : 2) : nil
     ^ Style/NestedTernaryOperator: Ternary operators must not be nested. Prefer `if` or `else` constructs instead.
