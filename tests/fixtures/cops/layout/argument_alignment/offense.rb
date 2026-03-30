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

# Misaligned &block argument (keyword hash kept as single item in with_first_argument)
comm.sudo(command,
  elevated: config.privileged,
  ^^^^^^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
  interactive: config.interactive,
  &handler
  ^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
)

# Misaligned &block with keyword args
builder.send(:collection,
  attribute_name, collection, value_method, label_method,
  ^^^^^^^^^^^^^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
  input_options, merged_input_options,
  ^^^^^^^^^^^^^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
  &collection_block
  ^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
)

# Misaligned block pass in a ternary branch call
(block ? capture(fields,
  &block) : render(partial_name, f: fields)) + fields.hidden_field(:_destroy)
  ^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.

# Same shape with a different continuation indent
(block ? capture(fields,
    &block) : render(partial_name, f: fields)) + fields.hidden_field(:_destroy)
    ^ Layout/ArgumentAlignment: Align the arguments of a method call if they span more than one line.
