if foo
^^ Style/SoleNestedConditional: Consider merging nested conditions into outer `if` conditions.
  if bar
    do_something
  end
end

unless foo
^^^^^^ Style/SoleNestedConditional: Consider merging nested conditions into outer `if` conditions.
  if bar
    do_something
  end
end

if x
^^ Style/SoleNestedConditional: Consider merging nested conditions into outer `if` conditions.
  unless y
    do_something
  end
end
