def func
  begin
    ala
  rescue => e
    bala
  end
  something
end

def bar
  ala
rescue => e
  bala
end

def baz
  do_something
end

# begin with rescue in assignment is NOT redundant
@value ||= begin
  compute_value
rescue => e
  fallback
end

# begin with multiple statements in assignment is NOT redundant
@value ||= begin
  setup
  compute_value
end
