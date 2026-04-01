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

# Single-line block if with `then`/`end`
def test
  if something then work end
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

# Parenthesized assignment used by a later bare expression in a multi-statement branch
def test
  if (deprecated_value = deprecated_options.delete(key))
    warn "deprecated"
    deprecated_value
  end
end

# Multi-assignment in condition used in body
def test
  if (var, obj = simple_comparison_lhs(node)) || (obj, var = simple_comparison_rhs(node))
    return if var.call_type?
    [var, obj]
  end
end

# Parenthesized ||= assignment in condition used in the non-guard branch
def test
  if (object ||= fallback) && object.respond_to?(:to_param)
    @auto_index = object.to_param
  else
    raise ArgumentError, object.inspect
  end
end

# Multiline heredoc guard branch is not a single-line branch guard clause
def test(database_id)
  if splitted = database_id.split(":") and splitted.length == 2
    splitted
  else
    fail(
      <<-TXT
        Expected database id '#{database_id}'
      TXT
    )
  end
end

# Assignment parent suppresses branch-style guard-clause suggestions
def test
  result = if something
    raise "error"
  else
    work
  end
end

# Multiline assignment parent suppresses branch-style guard-clause suggestions
def test
  result =
    if something
      raise "error"
    else
      work
    end
end

# Assignment in condition used in the non-guard branch
def test
  if (foo = bar)
    return foo
  else
    baz
  end
end

# If-else where else branch is comment-only (no code) — not flagged by RuboCop
# because Parser gem treats comment-only else as no-else
def test
  if condition
    raise "error"
  else
    # just a comment
  end
end

# Setter assignment parent suppresses branch-style guard-clause suggestions
def test(obj)
  obj.value = if something
    raise "error"
  else
    work
  end
end
