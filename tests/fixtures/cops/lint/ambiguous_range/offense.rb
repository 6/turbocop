x || 1..2
^^^^^^ Lint/AmbiguousRange: Wrap complex range boundaries with parentheses to avoid ambiguity.

x || 1..y || 2
^^^^^^ Lint/AmbiguousRange: Wrap complex range boundaries with parentheses to avoid ambiguity.
        ^^^^^^ Lint/AmbiguousRange: Wrap complex range boundaries with parentheses to avoid ambiguity.

1..2.to_a
   ^^^^^^ Lint/AmbiguousRange: Wrap complex range boundaries with parentheses to avoid ambiguity.

a + 1..b - 1
^^^^^ Lint/AmbiguousRange: Wrap complex range boundaries with parentheses to avoid ambiguity.
       ^^^^^ Lint/AmbiguousRange: Wrap complex range boundaries with parentheses to avoid ambiguity.

x * 2..y
^^^^^ Lint/AmbiguousRange: Wrap complex range boundaries with parentheses to avoid ambiguity.
