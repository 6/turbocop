a do
  b
end.c
    ^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

foo do |x|
  x
end.count
    ^^^^^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

items.each do |i|
  i.process
end.size
    ^^^^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.
