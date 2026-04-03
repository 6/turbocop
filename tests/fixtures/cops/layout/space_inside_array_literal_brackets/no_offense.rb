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
# Multiline array: space after [ is accepted when the next line starts with a comment
agents = [ 
  # comment
  "a"
]
# Array pattern: trailing comma before ] suppresses bracket spacing offenses
case [0]
in [  a  , ]
  a
end
# Constant array pattern: trailing comma before ] also suppresses bracket spacing offenses
case x
in Foo[ a, ]
  1
end
