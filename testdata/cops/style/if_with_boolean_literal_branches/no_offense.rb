if condition
  true
else
  false
end

unless condition
  false
else
  true
end

if foo == bar
  do_something
else
  false
end

if foo && bar
  true
else
  false
end

if foo || bar
  true
else
  false
end

# Ternary with non-boolean condition (hash lookup, variable, method returning non-boolean)
instance_options[:relationships].following[object.id] ? true : false
Redis::Alfred.set(key, value, nx: true, ex: timeout) ? true : false
app ? true : false

# Ternary with non-boolean method (no ? suffix)
foo.do_something ? true : false
