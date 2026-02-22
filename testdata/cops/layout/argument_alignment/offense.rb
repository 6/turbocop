foo(1,
  2,
  ^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
  3)
  ^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
bar(:a,
      :b,
      ^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
      :c)
      ^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
baz("x",
        "y")
        ^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.

obj.set :foo => 1234,
    :bar => 'Hello World',
    ^^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
    :baz => 'test'
    ^^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.

Klass[:a => :a, :b => :b,
  :c => :c,
  ^^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
  :d => :d]
  ^^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
