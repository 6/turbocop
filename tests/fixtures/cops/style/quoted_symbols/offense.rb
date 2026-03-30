:"foo"
^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

:"hello world"
^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

:"bar_baz"
^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "name": 'val' }
  ^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "role": 1, "color": 2 }
  ^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.
             ^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "\\": [ "\\" ] }
  ^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

:"symbols__\\",
^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

when :"\\"
     ^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "not_allowed(_\\d)?": false }
  ^^^^^^^^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "allowed_\\d": true }
  ^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "dependent_schema(_\\d)?": true }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.
