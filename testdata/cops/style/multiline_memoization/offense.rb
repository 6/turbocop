foo ||= (
^^^^^^^^^ Style/MultilineMemoization: Wrap multiline memoization blocks in `begin` and `end`.
  bar
  baz
)

foo ||=
^^^^^^^ Style/MultilineMemoization: Wrap multiline memoization blocks in `begin` and `end`.
  (
    bar
    baz
  )

foo ||= (bar ||
^^^^^^^^^^^^^^^ Style/MultilineMemoization: Wrap multiline memoization blocks in `begin` and `end`.
          baz)
