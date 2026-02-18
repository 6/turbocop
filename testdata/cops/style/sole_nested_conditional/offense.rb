if foo
  if bar
  ^^ Style/SoleNestedConditional: Consider merging nested conditions into outer `if` conditions.
    do_something
  end
end

unless foo
  if bar
  ^^ Style/SoleNestedConditional: Consider merging nested conditions into outer `if` conditions.
    do_something
  end
end

if x
  unless y
  ^^^^^^ Style/SoleNestedConditional: Consider merging nested conditions into outer `if` conditions.
    do_something
  end
end
