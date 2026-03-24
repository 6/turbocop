[1, 2, 3,]
        ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.

["a", "b",]
         ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.

[:foo, :bar,]
           ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.

# nitrocop-expect: 10:3 Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.
# Multiline array with trailing comma and blank line before closing bracket
[
  1,
  2,

]

# nitrocop-expect: 17:5 Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.
# Multiline array with trailing comma and comment before closing bracket
[
  "x",
  "y", # a comment

]

# Heredoc as last element with trailing comma (FN fix)
# nitrocop-expect: 24:14 Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.
x = [
  "foo",
  <<~STR.chomp,
    content here
  STR
]

# Heredoc as last element with trailing comma (no method chain)
# nitrocop-expect: 32:8 Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.
y = [
  "bar",
  <<~STR,
    more content
  STR
]

# Heredoc with squiggly heredoc and trailing comma
# nitrocop-expect: 40:12 Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array.
z = [
  "baz",
  <<~HEREDOC,
    some text
  HEREDOC
]
