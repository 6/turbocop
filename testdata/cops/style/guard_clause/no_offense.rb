# Already a guard clause (modifier form)
def test
  return unless something
  work
end

# Already a guard clause (modifier form)
def test
  return if something
  work
end

# Single-line modifier if
def test
  work if something
end

# If-else at end of method (allowed)
def test
  if something
    work
  else
    other_work
  end
end

# Ternary (not flagged)
def test
  something ? work : other_work
end

# Empty method body
def test
end
