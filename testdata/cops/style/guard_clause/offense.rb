def test
  if something
  ^^ Style/GuardClause: Use a guard clause (`return unless something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  unless something
  ^^^^^^ Style/GuardClause: Use a guard clause (`return if something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  other_work
  if something
  ^^ Style/GuardClause: Use a guard clause (`return unless something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  other_work
  unless something
  ^^^^^^ Style/GuardClause: Use a guard clause (`return if something`) instead of wrapping the code inside a conditional expression.
    work
  end
end
