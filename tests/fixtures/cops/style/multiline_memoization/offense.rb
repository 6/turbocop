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

@info["exif"] ||= (
^^^^^^^^^^^^^^^^^^^^^ Style/MultilineMemoization: Wrap multiline memoization blocks in `begin` and `end`.
  hash = {}
  output = self["%[EXIF:*]"]
  hash
)

foo.bar ||= (
^^^^^^^^^^^^^ Style/MultilineMemoization: Wrap multiline memoization blocks in `begin` and `end`.
  x = 1
  y = 2
)
