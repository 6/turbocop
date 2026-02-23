a = *[1, 2, 3]
    ^^^^^^^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

a = *'a'
    ^^^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.

a = *1
    ^^ Lint/RedundantSplatExpansion: Replace splat expansion with comma separated values.
