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

def qux?
  nil
  ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
end

def quux?
  return true if condition

  nil
  ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
end

def size?(file_name)
  entry = mapped_zip.find_entry(file_name)
  entry.nil? || entry.directory? ? nil : entry.size
                                   ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
end

def corge?
  if bar
    nil
    ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
  else
    true
  end
end

def grault?
  unless bar
    nil
    ^^^ Style/ReturnNilInPredicateMethodDefinition: Return `false` instead of `nil` in predicate methods.
  else
    true
  end
end
