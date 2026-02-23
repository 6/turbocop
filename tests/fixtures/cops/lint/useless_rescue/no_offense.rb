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

def with_ensure
  do_something
rescue => e
  raise
ensure
  do_cleanup(e)
end

def with_ensure_raise_var
  do_something
rescue => e
  raise e
ensure
  log(e)
end
