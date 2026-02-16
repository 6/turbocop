if foo
  bar
else
  baz
end

if foo
  bar
elsif qux
  baz
else
  quux
end

x = true ? 1 : 2

# Assignment context (variable style): else aligns with LHS
x = if foo
  bar
else
  baz
end

result = if condition
  value_a
elsif other
  value_b
else
  value_c
end

# Assignment context (keyword style): else aligns with if
links = if enabled?
          bar
        else
          baz
        end
