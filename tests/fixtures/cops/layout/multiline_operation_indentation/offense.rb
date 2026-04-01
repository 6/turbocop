x = 1 +
        2
        ^^^ Layout/MultilineOperationIndentation: Use 2 (not 8) spaces for indentation of a continuation line.
z = 5 +
      6
      ^^^ Layout/MultilineOperationIndentation: Use 2 (not 6) spaces for indentation of a continuation line.
w = a &&
         b
         ^^^^ Layout/MultilineOperationIndentation: Align the operands of an expression in an assignment spanning multiple lines.

# Chained || with same-indent continuations (most common FN pattern)
def skip?
  a ||
  b ||
  ^ Layout/MultilineOperationIndentation: Use 2 (not 0) spaces for indenting an expression spanning multiple lines.
  c
  ^ Layout/MultilineOperationIndentation: Use 2 (not 0) spaces for indenting an expression spanning multiple lines.
end

# Multiline && in if condition - misaligned
if a &&
  b
  ^ Layout/MultilineOperationIndentation: Align the operands of a condition in an `if` statement spanning multiple lines.
  do_something
end
