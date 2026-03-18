if x == 1
  do_something
end

while (y = gets)
  process(y)
end

if condition
  result
end

x = 1
if x
  foo
end

# ||= is conditional assignment, not flagged
raise StandardError unless foo ||= bar

# &&= is conditional assignment, not flagged
x &&= y if condition

# assignment inside a block in condition is not flagged
return 1 if any_errors? { o = inspect(file) }

# assignment inside a block followed by method call
return 1 if any_errors? { o = file }.present?

# assignment in a block after ||
if x?(bar) || y? { z = baz }
  foo
end

# empty condition
if ()
end

unless ()
end

# safe assignment: parenthesized
if (test = 10)
end

if (test[0] = 10)
end

# safe compound assignment inside parentheses
if (test = foo && bar == baz)
end

if (test = foo || bar == baz)
end

# assignment inside method call arguments is not flagged
return unless %i[asc desc].include?(order = params[:order])

# safe assignment with constant path write (parenthesized)
if (Foo::Bar = load_config)
end

# begin..end while/until with assignment in condition is not flagged
# (while_post / until_post in parser gem — RuboCop's on_while doesn't fire for these)
begin
  buffer += data
end while data = read_next

begin
  buffer += parts
end until parts = fetch_data

begin
  line.concat(c)
end while c = getc

# assignment in case/when body outside of a condition is not flagged
case kind
when :special
  result = lookup(kind)
else
  result = default_value
end

# assignment in when condition outside of a condition is not flagged
case
when match = scan(/foo/)
  process(match)
end
