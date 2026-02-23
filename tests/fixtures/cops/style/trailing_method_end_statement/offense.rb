def some_method
do_stuff; end
          ^^^ Style/TrailingMethodEndStatement: Place the end statement of a multi-line method on its own line.

def do_this(x)
  baz.map { |b| b.this(x) } end
                            ^^^ Style/TrailingMethodEndStatement: Place the end statement of a multi-line method on its own line.

def foo
  block do
    bar
  end end
      ^^^ Style/TrailingMethodEndStatement: Place the end statement of a multi-line method on its own line.
