def something
  x = Something.new
  x.attr = 5
  x
end

def another
  @obj.name = 'foo'
end

def third
  obj = Object.new
  obj.name = 'foo'
  do_something(obj)
end
