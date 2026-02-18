unless a && b || c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless x || y && z
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless foo && bar || baz
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end
