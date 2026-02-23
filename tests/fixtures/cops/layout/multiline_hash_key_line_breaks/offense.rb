x = {
  a: 1, b: 2,
        ^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  c: 3
}

y = {
  foo: :bar, baz: :qux,
             ^^^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  quux: :corge
}

z = {
  one: 1, two: 2,
          ^^^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  three: 3
}
