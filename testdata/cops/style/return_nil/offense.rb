def foo
  return nil
  ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
end

def bar
  return nil if something
  ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
end

def baz
  return nil unless condition
  ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
end
