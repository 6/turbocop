def foo?
  return if condition
  ^^^^^^ Style/ReturnNilInPredicateMethodDefinition: Avoid using `return nil` or `return` in predicate methods.
end

def bar?
  return nil
  ^^^^^^^^^^ Style/ReturnNilInPredicateMethodDefinition: Avoid using `return nil` or `return` in predicate methods.
end

def baz?
  return unless x
  ^^^^^^ Style/ReturnNilInPredicateMethodDefinition: Avoid using `return nil` or `return` in predicate methods.
end
