x = if condition
  do_something
else
  do_other_thing
end

x = condition ? 1 : 2
y = if flag
  compute_a
else
  compute_b
end
z = 42

# case/when - RuboCop does NOT flag case expressions
x = case value
when 1
  x
else
  :something
end

x = case value
when :a
  :b
when :c
  x
else
  :d
end

# elsif - RuboCop does NOT flag if/elsif/else even if one branch self-assigns
foo = if condition
  foo
elsif another_condition
  bar
else
  baz
end

foo = if condition
  bar
elsif another_condition
  foo
else
  baz
end

foo = if condition
  bar
elsif another_condition
  baz
else
  foo
end

# Multiline branches - not flagged when branch has multiple statements
foo = if condition
  bar
  baz
else
  foo
end

foo = if condition
  foo
else
  bar
  baz
end

# Instance variables - RuboCop does NOT flag these
@foo = condition ? @bar : @foo

# Class variables - RuboCop does NOT flag these
@@foo = condition ? @@bar : @@foo

# Global variables - RuboCop does NOT flag these
$foo = condition ? $bar : $foo

# Only if branch (no else)
foo = if condition
  bar
end

# Method call lhs - not a local variable
foo.do_something = condition ? foo.do_something : bar.do_something

# Multi-assignment
foo, bar = baz

# Ternary branches wrapped in parentheses - RuboCop does NOT flag these
foo = condition ? foo : (bar)
foo = condition ? (foo) : bar

sub_model = condition ? (json.send(:eval, sub_model) rescue nil) : sub_model
opts = cond ? (resource_name.kind_of?(Hash) ? resource_name : {}) : opts
