a = *[1, 2, 3]
    ^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

a = *'a'
    ^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

a = *1
    ^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

# Percent literal splat inside an array literal (method arg) — NOT exempt
foo([*%w[a b c]])
     ^^^^^^^^^ Lint/RedundantSplatExpansion: Pass array contents as separate arguments.

bar([*%W[x y], z])
     ^^^^^^^ Lint/RedundantSplatExpansion: Pass array contents as separate arguments.

baz([*%i[a b c]])
     ^^^^^^^^^ Lint/RedundantSplatExpansion: Pass array contents as separate arguments.

# when clause with percent literal splat — not a method argument, not exempt
case x
when *%w[foo bar baz]
     ^^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.
  1
end

# Array.new splat in assignment
a = *Array.new(3) { 42 }
    ^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

# Array.new splat with ::Array
a = *::Array.new(3) { 42 }
    ^^^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

# Single-element array literal with Array.new in assignment context
a = [*Array.new(foo)]
     ^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

# Array.new inside [] method call in assignment context — flagged
ns = NoteSet[*Array.new(n) { |i| notes[(d + (i * 2)) % size] }]
             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

# Block form inside a multi-element array literal is still an offense
schema = [
  1,
  *Array.new(3) { |i| i },
  ^^^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.
  2,
]

# Nested call that is the direct RHS of an assignment is still an offense
result = obj.call(*Array.new(5) { [] })
                  ^^^^^^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

# Direct assignment RHS method call from the corpus
link1 = fu_clean_components(*Array.new([n, 0].max, '..'), *srcdirs[i..-1])
                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.
