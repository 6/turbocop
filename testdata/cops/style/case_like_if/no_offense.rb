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
