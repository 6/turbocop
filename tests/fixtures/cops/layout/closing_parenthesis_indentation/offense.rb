some_method(
  a,
  b
    )
    ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 4).

some_method(
  a,
  b
      )
      ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 6).

other_method(
  x,
  y
        )
        ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 8).

# Grouped expression with hanging )
w = x * (
  y + z
  )
  ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 2).

# Nested call: first arg on next line, `)` under-indented
class Foo
  def bar
    method_call(
      arg1,
      arg2
        )
        ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 4 (not 8).
  end
end

# Scenario 2 with args on same line: `)` should align with `(`
some_method(a
)
^ Layout/ClosingParenthesisIndentation: Align `)` with `(`.

# Def with first param on same line: `)` should align with `(`
def some_method(a
)
^ Layout/ClosingParenthesisIndentation: Align `)` with `(`.
end

# No-args call with hanging paren: `)` misaligned
some_method(
    )
    ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 4).

# Def with no params: `)` misaligned
def some_method(
    )
    ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 4).
end

# Scenario 2: aligned args, `)` not aligned with `(`
some_method(a,
            b,
            c
)
^ Layout/ClosingParenthesisIndentation: Align `)` with `(`.

# Scenario 2: unaligned args, `)` misindented
some_method(a,
  x: 1,
  y: 2
              )
              ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 14).

# Indented no-args call: `)` misaligned
class Foo
  def bar
    some_method(
        )
        ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 4 (not 8).
  end
end

# Method assignment: no args, `)` misaligned
foo = some_method(
  )
  ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 2).

# Grouped expression with first operand on same line: `)` should align with `(`
if ((foo) &&
    (bar)
)
^ Layout/ClosingParenthesisIndentation: Align `)` with `(`.
  baz
end

# Heredoc grouped expression with first operand on same line: `)` should align with `(`
recipes = {
  a: (<<EOF
hello
EOF
  ),
  ^ Layout/ClosingParenthesisIndentation: Align `)` with `(`.
}

# String interpolation with first expression on its own line: `}` should outdent
message = "foo #{
  bar
  }"
  ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 0 (not 2).

# String interpolation with first expression on same line: `}` should align with `#{`
message = "foo #{bar
  }"
  ^ Layout/ClosingParenthesisIndentation: Align `)` with `(`.

# Nested interpolation: expected column should use the surrounding line indentation
class Foo
  def bar
    message = "foo #{
      baz
      }"
      ^ Layout/ClosingParenthesisIndentation: Indent `)` to column 4 (not 6).
  end
end
