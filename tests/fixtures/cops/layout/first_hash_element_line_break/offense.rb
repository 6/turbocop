x = { a: 1,
      ^ Layout/FirstHashElementLineBreak: Add a line break before the first element of a multi-line hash.
  b: 2,
  c: 3
}

y = { foo: :bar,
      ^^^ Layout/FirstHashElementLineBreak: Add a line break before the first element of a multi-line hash.
  baz: :qux
}

z = { one: 1,
      ^^^ Layout/FirstHashElementLineBreak: Add a line break before the first element of a multi-line hash.
  two: 2,
  three: 3
}
