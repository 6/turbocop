if condition_a
  action_a
else
  if condition_b
  ^^ Style/IfInsideElse: Convert `if` nested inside `else` to `elsif`.
    action_b
  else
    action_c
  end
end
if a
  1
else
  if b
  ^^ Style/IfInsideElse: Convert `if` nested inside `else` to `elsif`.
    2
  end
end
if x
  foo
else
  if y
  ^^ Style/IfInsideElse: Convert `if` nested inside `else` to `elsif`.
    bar
  else
    baz
  end
end
