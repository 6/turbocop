if foo && bar
  do_something
end

if foo
  if bar
    do_something
  else
    other_thing
  end
end

if foo
  do_something
  do_other_thing
end
x = 1

# Variable assignment in outer condition used by inner condition
if var = foo
  do_something if var
end

if cond && var = foo
  do_something if var
end

if var = compute_value
  if var
    process(var)
  end
end

# Variable assigned in outer condition inside a comparison - used in inner modifier
def with_locale_param(url_hash)
  if (locale = some_method) != default_locale
    url_hash.update :locale => locale if locale
  end
  url_hash
end

# Multi-write destructuring in outer condition - assigned var used in inner modifier
if directives && (result, = directives['call-seq'])
  process(result) if result
end
