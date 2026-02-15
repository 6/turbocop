foo
  .bar
    .baz
    ^^^ Layout/MultilineMethodCallIndentation: Align `.` with `.` on the previous line of the chain.

thing
  .first
  .second
      .third
      ^^^ Layout/MultilineMethodCallIndentation: Align `.` with `.` on the previous line of the chain.

query
  .select('foo')
  .where(x: 1)
    .order(:name)
    ^^^ Layout/MultilineMethodCallIndentation: Align `.` with `.` on the previous line of the chain.
