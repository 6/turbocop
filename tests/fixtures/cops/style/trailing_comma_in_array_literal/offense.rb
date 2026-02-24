[1, 2, 3,]
        ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.

["a", "b",]
         ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.

[:foo, :bar,]
           ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.

# turbocop-expect: 10:3 Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.
# Multiline array with trailing comma and blank line before closing bracket
[
  1,
  2,

]

# turbocop-expect: 17:5 Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.
# Multiline array with trailing comma and comment before closing bracket
[
  "x",
  "y", # a comment

]
