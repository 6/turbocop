def foo
  return 42
  ^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
end

def bar
  x = 1
  return x
  ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
end

def baz(x)
  return x + 1
  ^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
end

# return in terminal position of if/else
def with_if(x)
  if x > 0
    return x
    ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  else
    return -x
    ^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  end
end

# return in terminal position of if/elsif/else
def with_elsif(x)
  if x > 0
    return 1
    ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  elsif x == 0
    return 0
    ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  else
    return -1
    ^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  end
end

# return in terminal position of case/when
def with_case(x)
  case x
  when 1
    return :one
    ^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  when 2
    return :two
    ^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  else
    return :other
    ^^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  end
end

# return in terminal position of begin/rescue
def with_rescue
  begin
    return do_something
    ^^^^^^^^^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  rescue StandardError
    return default_value
    ^^^^^^^^^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  end
end

# return in terminal position of unless
def with_unless(x)
  unless x.nil?
    return x
    ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  else
    return 0
    ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  end
end

# return in nested if inside case
def nested_control(x)
  case x
  when :a
    if true
      return 1
      ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
    else
      return 2
      ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
    end
  else
    return 3
    ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  end
end

# return in begin/rescue/else/ensure - rescue is the body's last statement
def with_rescue_else
  begin
    return try_something
    ^^^^^^^^^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  rescue
    return fallback
    ^^^^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
  end
end

# implicit begin (def body with rescue)
def implicit_rescue
  return do_work
  ^^^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
rescue
  return safe_value
  ^^^^^^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
end
