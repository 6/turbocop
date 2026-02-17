def foo
  do_something
rescue
^^^^^^ Lint/UselessRescue: Useless `rescue` detected.
  raise
end

def bar
  do_something
rescue => e
^^^^^^ Lint/UselessRescue: Useless `rescue` detected.
  raise e
end

def baz
  do_something
rescue
^^^^^^ Lint/UselessRescue: Useless `rescue` detected.
  raise $!
end
