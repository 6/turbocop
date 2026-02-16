def some_method(used, _unused)
  puts used
end

def no_args
  puts "hello"
end

def empty_method(unused)
end

def not_implemented(unused)
  raise NotImplementedError
end

def not_implemented2(unused)
  fail "TODO"
end

def all_used(a, b)
  a + b
end
