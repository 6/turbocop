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

# elsif without final else — offense on the last elsif
if x > 1
  bar
elsif x < 0
^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  baz
end

# Multiple elsif without final else — offense on LAST elsif only
if cond_1
  one
elsif cond_2
  two
elsif cond_3
^^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  three
end
