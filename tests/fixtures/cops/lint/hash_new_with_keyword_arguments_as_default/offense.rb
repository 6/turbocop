Hash.new(key: :value)
         ^^^^^^^^^^^ Lint/HashNewWithKeywordArgumentsAsDefault: Use a hash literal instead of keyword arguments.
Hash.new(foo: 1, bar: 2)
         ^^^^^^^^^^^^^^^ Lint/HashNewWithKeywordArgumentsAsDefault: Use a hash literal instead of keyword arguments.
::Hash.new(a: 1)
           ^^^^ Lint/HashNewWithKeywordArgumentsAsDefault: Use a hash literal instead of keyword arguments.
