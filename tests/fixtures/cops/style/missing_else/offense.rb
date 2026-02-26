if cond
^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  foo
end

if x > 1
^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  bar
end

case status
^^^^^^^^^^^ Style/MissingElse: `case` condition requires an `else`-clause.
when :active
  activate
end
