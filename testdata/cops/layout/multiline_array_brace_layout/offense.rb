x = [:a,
  :b
]
^ Layout/MultilineArrayBraceLayout: The closing array brace must be on the same line as the last array element when the opening brace is on the same line as the first array element.

y = [
  :a,
  :b]
    ^ Layout/MultilineArrayBraceLayout: The closing array brace must be on the line after the last array element when the opening brace is on a separate line from the first array element.

z = [:c,
  :d
]
^ Layout/MultilineArrayBraceLayout: The closing array brace must be on the same line as the last array element when the opening brace is on the same line as the first array element.
