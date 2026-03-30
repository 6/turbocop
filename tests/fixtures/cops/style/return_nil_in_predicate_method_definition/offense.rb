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

def implicit_nil?
  nil
  ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
end

def implicit_nil_with_guard?
  return true if condition
  nil
  ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
end

def nil_in_if_branch?
  if bar
    nil
    ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
  else
    true
  end
end

def nil_in_else_branch?
  if bar
    true
  else
    nil
    ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
  end
end

def nil_in_nested_if?
  if bar
    if baz
      true
    else
      nil
      ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
    end
  end
end

def nil_in_ternary?
  entry.nil? || entry.directory? ? nil : entry.size
                                   ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
end
