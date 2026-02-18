if cond
^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  foo
end

if x > 1
^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  bar
elsif x < 0
  baz
end

case status
^^^^^^^^^^^ Style/MissingElse: `case` condition requires an `else`-clause.
when :active
  activate
end
