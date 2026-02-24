if x > 1
  do_something
end

while x > 1
  do_something
end

until x > 1
  do_something
end

x > 1 ? a : b

if x && y
  do_something
end

# AllowSafeAssignment: true (default)
if (a = something)
  use(a)
end

while (line = gets)
  process(line)
end

if (result = compute)
  handle(result)
end

# Setter method call is a safe assignment
if (self.name = value)
  use(name)
end

while (node.parent = next_node)
  process(node)
end

# No space between keyword and paren (parens_required)
if(x > 5) then something end
do_something until(x > 5)
while(running)
  process
end
unless(done)
  keep_going
end

# Ternary with parentheses in condition is fine
(a == 0) ? b : a

# Leading parenthesized subexpression
(a > b) && other ? one : two

# Parentheses around subexpression, not the whole condition
if (a + b).c()
end

# Modifier conditional inside parens
if (something rescue top)
end

if (something if cond)
end

if (something unless cond)
end

if (something while cond)
end

if (something until cond)
end

# Semicolon-separated expressions inside parens
if (foo; bar)
  do_something
end

# Empty parens
if ()
end

unless ()
end

# Element assignment (safe assignment via setter)
if (test[0] = 10)
end

# while/until with do..end block in condition
while (foo do
      end)
end

until (foo do
      end)
end
