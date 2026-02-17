def some_method
  do_stuff
end

def do_this(x)
  baz.map { |b| b.this(x) }
end

def foo
  block do
    bar
  end
end

def single_line; end
