x = { a: 1,
  b: 2
}
^ Layout/MultilineHashBraceLayout: Closing hash brace must be on the same line as the last hash element when opening brace is on the same line as the first hash element.

y = {
  a: 1,
  b: 2 }
       ^ Layout/MultilineHashBraceLayout: Closing hash brace must be on the line after the last hash element when opening brace is on a separate line from the first hash element.

z = { c: 3,
  d: 4
}
^ Layout/MultilineHashBraceLayout: Closing hash brace must be on the same line as the last hash element when opening brace is on the same line as the first hash element.
