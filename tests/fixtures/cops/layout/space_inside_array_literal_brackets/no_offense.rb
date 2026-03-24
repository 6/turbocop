[1, 2, 3]
[:a, :b]
[]
x = ["foo"]
[1,
 2,
 3]
# Multiline with ] on its own line (end_ok)
[1,
 2,
 3
]
# Elements on next line (start_ok for no_space: not a comment, so would flag if space)
[
  1,
  2
]
