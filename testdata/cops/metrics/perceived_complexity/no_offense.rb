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
  else
    0
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

def single_case(x)
  case x
  when 1
    :one
  when 2
    :two
  end
end

# Multiple rescue clauses count as a single decision point
def multiple_rescues(x)
  if x > 0
    1
  else
    0
  end
  if x > 1
    2
  end
  while x > 10
    x -= 1
  end
  begin
    risky
  rescue ArgumentError
    handle_arg
  rescue TypeError
    handle_type
  rescue StandardError
    handle_std
  end
end

# loop do...end blocks do not count toward complexity (not an iterating method)
def method_with_loop
  if a
    1
  end
  if b
    2
  end
  if c
    3
  end
  loop do
    if d
      break
    end
    if e
      next
    end
    if f
      return
    end
  end
end
