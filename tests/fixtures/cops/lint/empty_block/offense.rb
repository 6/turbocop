items.each { |x| }
^ Lint/EmptyBlock: Empty block detected.

items.each do |x|
^ Lint/EmptyBlock: Empty block detected.
end

foo { }
^ Lint/EmptyBlock: Empty block detected.

Context.create_table(:users) do |t|
^ Lint/EmptyBlock: Empty block detected.
  t.timestamps null: false
end.define_model do
end

super(name, extensions: extensions, block: block, **kwargs) {}
^^^^^ Lint/EmptyBlock: Empty block detected.

block_given? ? super : super {}
                       ^^^^^^ Lint/EmptyBlock: Empty block detected.

super {}
^^^^^^ Lint/EmptyBlock: Empty block detected.

    super {}
    ^^^^^^ Lint/EmptyBlock: Empty block detected.
