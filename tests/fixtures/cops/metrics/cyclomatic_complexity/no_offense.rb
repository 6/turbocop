def simple_method
  if x
    1
  end
end

def no_branches
  a = 1
  b = 2
  a + b
end

def moderate(x)
  if x > 0
    1
  end
  if x > 1
    2
  end
  while x > 10
    x -= 1
  end
end

def empty_method
end

def single_rescue
  begin
    risky
  rescue StandardError
    fallback
  end
end

# Multiple rescue clauses count as a single decision point (score = 1 + 1 = 2)
def multiple_rescues
  begin
    x if condition1
    y if condition2
    z if condition3
    w if condition4
    v if condition5
    risky
  rescue ArgumentError
    handle_arg
  rescue TypeError
    handle_type
  rescue StandardError
    handle_std
  end
end
