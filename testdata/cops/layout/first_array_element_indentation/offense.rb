x = [
      1,
      ^^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the start of the line where the left square bracket is.
  2,
  3
]
y = [
    4,
    ^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the start of the line where the left square bracket is.
  5
]
z = [
        6,
        ^^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the start of the line where the left square bracket is.
  7
]
# Closing bracket on own line with wrong indentation inside method call parens
foo([
      :bar,
      :baz
  ])
  ^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.
