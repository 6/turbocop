# Block if with empty line after multiline condition
if foo &&
   bar

  do_something
end

# Single line if condition
if foo && bar
  do_something
end

# Single line while condition
while condition
  do_something
end

# Block while with empty line after multiline condition
while multiline &&
   condition

  do_something
end

# Block until with empty line after multiline condition
until multiline ||
   condition

  do_something
end

# elsif with empty line after multiline condition
if condition
  do_something
elsif multiline &&
   condition

  do_something_else
end

# Modifier if with empty line after multiline condition
do_something if multiline &&
                condition

do_something_else

# Modifier if at last position (no right sibling) — no offense
def m
  do_something if multiline &&
                condition
end

# Modifier while at last position (no right sibling) — no offense
def m
  begin
    do_something
  end while multiline &&
        condition
end

# Modifier unless at top level with no right sibling — no offense
do_something unless multiline &&
                    condition

# Single line if at top level
do_something if condition

# case/when with empty line after multiline condition
case x
when foo,
    bar

  do_something
end

# case/when with single line condition
case x
when foo, bar
  do_something
end

# rescue with empty line after multiline exceptions
begin
  do_something
rescue FooError,
  BarError

  handle_error
end

# rescue with single line exceptions
begin
  do_something
rescue FooError
  handle_error
end

# Modifier if where keyword is at end of line but predicate is single-line
# RuboCop checks condition.multiline? (predicate first_line vs last_line)
raise ArgumentError, "bad index" if
  index > size && index < max

# Modifier unless where keyword is at end of line but predicate is single-line
do_something unless
  condition_met?

# Block if where keyword is at end of line but predicate is single-line
if
  some_condition
  do_something
end

# Ternary if — no offense even if condition is multiline (rare but possible)
x = (a &&
  b) ? 1 : 2

# Block if with whitespace-only line after multiline condition (treated as blank)
if helpers_data['x'] &&
   helpers_data['y'] &&
   helpers_data['z']

  puts "found"
end

# elsif with case expression as predicate — case is inherently multiline
if x
  foo
elsif case states.last
      when :initial, :media
        scan(/foo/)
      end
  bar
end

# Modifier if with only comment after (no right sibling in AST)
def m
  true if depth >= 3 &&
          caller.first.label == name
          # TODO: incomplete
end

# Modifier unless inside when block — when is not a right sibling
case parent
when Step
  return render_403 unless can_read_module?(protocol) ||
                           can_read_repository?(protocol)
when Result
  return render_403 unless can_read_result?(parent)
end

# elsif with bare case expression (no subject)
if x
  foo
elsif case
      when match = scan(/foo/)
        bar
      end
  baz
end
