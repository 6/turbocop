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

# Repeated &. on the same local variable are discounted (only first counts)
# Max: 8 (default). Base 1 + if 1 + first &. on obj 1 = 3 <= 8.
# Without discount, 8 &. calls would give base 1 + if 1 + 8 &. = 10 > 8.
def method_with_repeated_csend
  if (obj = find_something)
    a = obj&.foo
    b = obj&.bar
    c = obj&.baz
    d = obj&.qux
    e = obj&.quux
    f = obj&.corge
    g = obj&.grault
    h = obj&.garply
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

# define_method with simple body should not fire
define_method(:simple_block) do |x|
  if x
    1
  end
end

# block_pass on non-iterating method should not count
def method_with_non_iterating_block_pass(items)
  items.send(&:to_s)
  if items.empty?
    1
  end
end

# each_line, each_byte, each_char, each_codepoint, rindex are NOT in
# RuboCop's KNOWN_ITERATING_METHODS — blocks on these should not count.
# sort_by! (with bang) is also not in the canonical list (sort_by without bang IS).
# With Max:8 (default), base 1 + 7 ifs = 8 <= 8. If each_line block were
# counted, it would be 9 > 8 and fire.
def method_with_non_iterating_each_line(data)
  if data.nil?
    return
  end
  if data.empty?
    return
  end
  data.each_line { |line| process(line) }
  data.each_byte { |b| handle(b) }
  data.each_char { |c| check(c) }
  data.each_codepoint { |cp| validate(cp) }
  data.rindex { |x| x > 0 }
  [3, 1, 2].sort_by! { |x| -x }
  if data.length > 10
    1
  end
  if data.length > 20
    2
  end
  if data.length > 30
    3
  end
  if data.length > 40
    4
  end
  if data.length > 50
    5
  end
end

# case/in pattern matching should not double-count.
# RuboCop counts each `in` branch as +1 individually (no CaseMatchNode formula).
# With Max:8 (default), base 1 + 3 ifs + 3 in-branches = 7 <= 8.
# If CaseMatchNode formula were also applied (0.8 + 0.2*3 = 1.4 -> 1), it
# would add an extra +1 making it 8, and with the individual in-branches
# that would be double-counted.
def method_with_case_in(value)
  if value.nil?
    return
  end
  if value.empty?
    return
  end
  if value.frozen?
    return
  end
  case value
  in Integer => n
    n
  in String => s
    s
  in Array => a
    a
  end
end
