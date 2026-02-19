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

# Misaligned end: else/elsif correctly aligned with `if` keyword
# (EndAlignment disabled scenario — end at arbitrary column)
x = if foo
      bar
    elsif qux
      baz
    else
      quux
    end

# Misaligned end: else aligned with `if` keyword (not end)
y = if condition
      value_a
    else
      value_b
    end
