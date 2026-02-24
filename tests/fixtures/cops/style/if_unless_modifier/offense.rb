if x
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  do_something
end

unless x
^^^^^^ Style/IfUnlessModifier: Favor modifier `unless` usage when having a single-line body.
  do_something
end

if condition
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  foo
end

unless finished?
^^^^^^ Style/IfUnlessModifier: Favor modifier `unless` usage when having a single-line body.
  retry
end

# Parenthesized condition (non-assignment) should still be flagged
if (x > 0)
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  do_something
end

# Blank line between condition and body should still be flagged
if condition
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.

  do_something
end

# One-line form should be flagged
if foo; bar; end
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
