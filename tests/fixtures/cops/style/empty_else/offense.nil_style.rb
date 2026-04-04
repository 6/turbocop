if condition
  do_something
else
^^^^ Style/EmptyElse: Redundant `else`-clause.
  nil
end

case foo
when :bar
  do_bar
else
^^^^ Style/EmptyElse: Redundant `else`-clause.
  nil
end
