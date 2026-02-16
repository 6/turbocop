x = [:a,
     ^^ Layout/FirstArrayElementLineBreak: Add a line break before the first element of a multi-line array.
  :b,
  :c
]

y = [1,
     ^ Layout/FirstArrayElementLineBreak: Add a line break before the first element of a multi-line array.
  2
]

z = [:foo,
     ^^^^ Layout/FirstArrayElementLineBreak: Add a line break before the first element of a multi-line array.
  :bar,
  :baz
]
