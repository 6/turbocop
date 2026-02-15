foo
  .bar
    .baz
    ^^^ Layout/MultilineMethodCallIndentation: Align `.baz` with `foo` on line 1.

thing
  .first
  .second
      .third
      ^^^ Layout/MultilineMethodCallIndentation: Align `.third` with `thing` on line 5.

query
  .select('foo')
  .where(x: 1)
    .order(:name)
    ^^^ Layout/MultilineMethodCallIndentation: Align `.order` with `query` on line 10.

# Block chain continuation: .sort_by should align with .with_index dot
frequencies.map.with_index { |f, i| [f / total, hex[i]] }
           .sort_by { |r| -r[0] }
           ^^^ Layout/MultilineMethodCallIndentation: Align `.sort_by` with `.with_index` on line 16.

# Hash pair value chain: chain inside hash value should align with chain root
foo(bar: baz
         .qux
           .quux)
           ^^^^ Layout/MultilineMethodCallIndentation: Align `.quux` with `baz` on line 20.
