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

# Multiline condition (if)
def test
  if something &&
     other_thing
    work
  end
end

# Multiline condition (unless)
def test
  unless something &&
         other_thing
    work
  end
end

# Assignment in condition used in body (if)
def test
  if (argument = destructuring_argument(args))
    corrector.replace(argument, argument.source)
  end
end

# Assignment in condition used in body (unless)
def test
  unless (result = compute_result(input))
    handle_missing(result)
  end
end

# Multi-assignment in condition used in body
def test
  if (var, obj = simple_comparison_lhs(node)) || (obj, var = simple_comparison_rhs(node))
    return if var.call_type?
    [var, obj]
  end
end
