:foo.to_sym
^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion detected.

:bar.to_sym
^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion detected.

'baz'.to_sym
^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion detected.

{ 'name': 'val' }
  ^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `name:` instead.

{ "role": 'val' }
  ^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `role:` instead.

{ 'status': 1, "color": 2 }
  ^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `status:` instead.
               ^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `color:` instead.
