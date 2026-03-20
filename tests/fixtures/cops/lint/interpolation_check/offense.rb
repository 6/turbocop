x = 'hello #{name}'
    ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

y = 'value: #{foo}'
    ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

z = '#{bar}'
    ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

w = 'THIS. IS. #{yield.upcase}!'
    ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

# String containing double quotes — RuboCop flags this (corrects with %{})
a = 'foo "#{bar}"'
    ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

# Split string where the second part has interpolation
"x" \
  'foo #{bar}'
  ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

# %q strings with interpolation patterns — RuboCop flags these (single-line only)
a = %q{hello #{name}}
    ^^^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

b = %q|text #{foo} bar|
    ^^^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

c = %q(value #{bar})
    ^^^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

# Strings with non-standard uppercase escapes like \A, \Z — the Parser gem
# does NOT fatally reject these (only \U is fatal). So these are valid
# interpolation candidates and should be flagged.
d = '\A#{regexp}\z'
    ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

e = '\Astring#{interpolation}\z'
    ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.
