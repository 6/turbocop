# Unnecessary to_sym on symbol literal
:foo.to_sym
^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo` instead.

# Unnecessary to_sym on string literal
"foo".to_sym
^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo` instead.

# Unnecessary to_sym on string with underscores
"foo_bar".to_sym
^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo_bar` instead.

# Unnecessary to_sym on string requiring quoting
"foo-bar".to_sym
^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"foo-bar"` instead.

# Unnecessary intern on symbol literal
:foo.intern
^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo` instead.

# Unnecessary intern on string literal
"foo".intern
^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo` instead.

# Unnecessary intern on string with underscores
"foo_bar".intern
^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo_bar` instead.

# Unnecessary intern on string requiring quoting
"foo-bar".intern
^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"foo-bar"` instead.

# Unnecessarily quoted standalone symbol (double quotes)
:"foo"
^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo` instead.

# Unnecessarily quoted standalone symbol (double quotes, underscore)
:"foo_bar"
^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo_bar` instead.

# Unnecessarily quoted standalone symbol (single quotes)
:'foo'
^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo` instead.

# Unnecessarily quoted standalone symbol (single quotes, underscore)
:'foo_bar'
^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo_bar` instead.

# Unnecessarily quoted operator symbol
obj.send(:"+")
         ^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:+` instead.

# Unnecessarily quoted instance variable symbol
instance_variable_get :"@ivar"
                      ^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:@ivar` instead.

# Quoted hash key (string style)
{ 'name': 'val' }
  ^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `name:` instead.

# Quoted hash key (double-quoted string style)
{ "role": 'val' }
  ^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `role:` instead.

# Multiple quoted hash keys
{ 'status': 1, "color": 2 }
  ^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `status:` instead.
               ^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `color:` instead.

# Quoted symbol as hash value
{ foo: :'bar' }
       ^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:bar` instead.

# Quoted symbol as hash key (rocket style)
{ :'foo' => :bar }
  ^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:foo` instead.

# Quoted hash key ending with !
{ 'foo!': 'bar' }
  ^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `foo!:` instead.

# Quoted hash key ending with ?
{ 'foo?': 'bar' }
  ^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `foo?:` instead.

# Interpolated string to_sym
"foo-#{bar}".to_sym
^^^^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"foo-#{bar}"` instead.

# Interpolated string intern
"foo-#{bar}".intern
^^^^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"foo-#{bar}"` instead.

# Uppercase quoted hash key
{ 'Foo': 1 }
  ^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `Foo:` instead.

# Double-quoted uppercase hash key
{ "Bar": 1 }
  ^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `Bar:` instead.

# Quoted hash key with underscore prefix
{ '_private': 1 }
  ^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `_private:` instead.

# Unnecessarily quoted numeric global variable symbol
:"$1"
^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:$1` instead.

# Unnecessarily quoted special global variable symbol
:"$?"
^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:$?` instead.

# Unnecessarily quoted special global symbol ($!)
:"$!"
^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:$!` instead.

# UTF-8 symbol that can be unquoted (Ruby allows multi-byte identifiers)
:"résumé"
^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:résumé` instead.

# UTF-8 single-quoted symbol
:'café'
^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:café` instead.

# UTF-8 hash key (colon-style)
{ 'naïve': 1 }
  ^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `naïve:` instead.

# Percent-string notation with interpolation and .to_sym
%(cover_#{face}_image).to_sym
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"cover_#{face}_image"` instead.

# Percent-string notation with leading interpolation and .to_sym
%(#{periphery}_background_color).to_sym
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"#{periphery}_background_color"` instead.

# Percent-string notation with interpolation and .intern
%(prefix_#{name}).intern
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"prefix_#{name}"` instead.

# Non-ASCII standalone symbol that can be unquoted (multiplication sign)
:"×"
^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:×` instead.

# Special global variable $$ (process ID)
:"$$"
^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:$$` instead.

# Non-UTF8 string to_sym
"\xff".to_sym
^^^^^^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:"\xFF"` instead.

# Percent-s with double-quote delimiter
%s"test"
^^^^^^^^ Lint/SymbolConversion: Unnecessary symbol conversion; use `:test` instead.
