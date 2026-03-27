case x
when 1
when 2
when 3
end

if x == 1
elsif x == 2
end

if x > 1
elsif x < 0
elsif x.nil?
end

# Different variables in each branch - not case-like
if x == 1
elsif y == 2
elsif z == 3
end

# Mixed comparison types with different targets
if x == 1
elsif y.is_a?(Integer)
elsif z === String
end

# Non-comparison conditions
if foo?
elsif bar?
elsif baz?
end

# Mixed-case constants are not literals (class references, not const references)
# RuboCop only treats ALL_UPPERCASE constants as literals
if cop == Foo::Bar
elsif cop == Baz::Qux
elsif cop == Something
else
  default_action
end

# match? with non-regexp should not be flagged (RuboCop requires regexp)
if x.match?(y)
elsif x.match?('str')
elsif x.match?(z)
end

# == with class reference on value side should not be flagged
if x == Foo
elsif Bar == x
elsif Baz == x
end

# One branch has == with class reference (mixed with literals)
if x == 1
elsif x == Foo
elsif x == 3
end

# == with method call arguments on both sides - not case-like
if x == foo(1)
elsif bar(1) == x
elsif baz(2) == x
end

# match? without a receiver
if match?(/foo/)
elsif x.match?(/bar/)
elsif x.match?(/baz/)
end

# unless should not be flagged (RuboCop skips unless)
unless x == 1
elsif x == 2
elsif x == 3
end

# include? without a receiver should not be flagged
if include?(Foo)
elsif include?(Bar)
elsif include?(Baz)
end

# cover? without a receiver should not be flagged
if x == 1
elsif cover?(Bar)
elsif x == 3
end

# Single-letter constant names should not count as const_reference
if x == F
elsif B == x
elsif C == x
end

# equal? without a receiver should not be flagged
if equal?(Foo)
elsif Bar == x
elsif x == 3
end

# Named captures in regexp with match should not be flagged
# case/when uses === which doesn't populate named capture locals
if foo.match(/(?<name>.*)/)
elsif foo == 123
elsif foo == 456
end

# Named captures in regexp with match (regexp as receiver)
if /(?<name>.*)/.match(foo)
elsif foo == 123
elsif foo == 456
end

# Named captures in a later branch should also prevent flagging
if foo == 1
elsif foo.match(/(?<capture>\d+)/)
elsif foo == 3
end

# kind_of? should NOT trigger case-when conversion (RuboCop only handles is_a?)
if range.kind_of?(Array)
elsif range.kind_of?(Time)
elsif range.kind_of?(String)
else
  raise "invalid"
end

# kind_of? mixed with other patterns should not be flagged either
if x.kind_of?(Integer)
elsif x.kind_of?(Float)
elsif x.kind_of?(String)
elsif x.kind_of?(Symbol)
end

# Safe navigation (&.) conditions are not convertible (RuboCop treats csend differently from send)
if default_pre == "'"
  :string
elsif default_pre&.match?(/^\d+$/)
  :integer
elsif default_pre&.match?(/^[A-z]+$/)
  :function
end

# Safe navigation in equality is also not convertible
if x == 1
elsif x&.==(2)
elsif x == 3
end

# Two branches + else (below default MinBranchesCount=3)
if data['key'] == 'phone'
elsif data['key'] == 'email'
else
  data['key']
end

# Else body with modifier unless — RuboCop walks branch_conditions into the
# else body (modifier unless is if_type in Parser AST), finds an unconvertible
# condition (start_with?), and rejects the chain.
if /^branches/.match?(line)
  nil
elsif /^revision/ =~ line
  do_something
elsif /^date/ =~ line
  author_utf8 = /author: ([^;]+)/.match(line_utf8)[1]
  file_state = /state: ([^;]+)/.match(line)[1]
else
  commit_log += line unless line.start_with?('*** empty log message ***')
end

# Else body with nested if-else — RuboCop walks branch_conditions into the
# nested if, finds an unconvertible condition (value.nil?), rejects the chain.
if condition[:pre_condition] == 'not_set'
  do_a
elsif condition[:pre_condition] == 'current_user.id'
  do_b
elsif condition[:pre_condition] == 'current_user.organization_id'
  do_c
else
  if condition[:value].nil?
    do_d
  else
    do_e
  end
end

# if-else with nested if-elsif in else body — NOT an elsif chain
# In Parser AST, else body wraps nested if in :begin, so branch_conditions stops.
# RuboCop does not walk from the outer if into a block if-else in the else body.
# (only modifier if/unless are walked into, since they are direct if_type in Parser AST)
if path == "*"
  true
else
  if path.is_a?(Regexp)
    path.match(stack[i])
  elsif path.is_a?(Symbol)
    path.inspect == stack[i]
  end
end

# if-else with nested if in else (regexp variant) — outer if is not case-like
if piped_row =~ /^\s+/
  last_step_params << piped_row
else
  if piped_row =~ /\=\=\=\s/
    :info
  elsif piped_row =~ /Build settings/
    :ignore
  end
end
