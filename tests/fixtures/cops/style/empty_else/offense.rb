if condition
  statement
else
^^^^ Style/EmptyElse: Redundant `else`-clause.
end

if condition
  statement
else
^^^^ Style/EmptyElse: Redundant `else`-clause.
  nil
end

case x
when 1
  "one"
else
^^^^ Style/EmptyElse: Redundant `else`-clause.
end
