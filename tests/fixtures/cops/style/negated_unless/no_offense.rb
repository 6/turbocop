unless x
  do_something
end

if !x
  do_something
end

if x
  do_something
end

do_something unless condition

unless x
  do_something
else
  do_other
end

# Double-bang (!! truthiness cast) is not a simple negation
unless !!enabled
  set_defaults
end

do_something unless !!active
