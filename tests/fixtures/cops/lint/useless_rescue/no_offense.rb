def foo
  do_something
rescue
  do_cleanup
  raise
end

def bar
  do_something
rescue => e
  log(e)
  raise e
end

def baz
  do_something
rescue ArgumentError
  raise
rescue
  # noop
end
