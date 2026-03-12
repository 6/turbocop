"hello #{name}"
'hello world'
'no interpolation here'
"value: #{foo}"
'literal string'
x = 'just a string'

# Heredoc with decorative single-quotes around interpolated values
msg = <<~MSG
  Database configuration specifies nonexistent '#{adapter_name}' adapter.
  Please install the '#{gem_name}' gem.
MSG

# Backtick strings with shell single-quoting
result = `git tag | grep '^#{tag}$'`

# Symbol with interpolation inside heredoc
code = <<~RUBY
  controller.send(:'#{method}', ...)
RUBY

# Mustache/Liquid template syntax that looks like interpolation
# but would be invalid Ruby if double-quoted
template = 'Created order #{{ response.order_number }} for {{ response.product }}'
url = 'https://example.com/users/{{ user_id }}/orders'

# String containing double quotes — converting to double-quoted would break syntax
f.puts 'gem "example", path: "#{File.dirname(__FILE__)}/../"'

# Format directive in interpolation-like pattern — not valid Ruby interpolation
msg = 'Replace interpolated variable `#{%<variable>s}`.'

# Escaped hash — backslash before # means not intended as interpolation
escaped = '\#{not_interpolation}'

# %w array — strings inside are not flagged
%w(#{a}-foo)

# Multiline single-quoted string where #{ and } are on different lines.
# RuboCop's regex /(?<!\\)#\{.*\}/ uses .* which doesn't cross newlines,
# so this is NOT flagged.
x = 'text #{
  some_value
}'

# BEGIN in interpolation — Parser gem rejects this as invalid syntax,
# so RuboCop does not flag it. Prism accepts it but we must match RuboCop.
msg = '#{BEGIN { setup }}'
txt = 'test #{BEGIN { x = 1 }}'

# \U escape — in single-quoted strings \U is literal backslash + U.
# When converted to double-quoted, Parser gem rejects \U as invalid escape.
# Prism accepts it, but we must match RuboCop behavior.
label = '\U+0041 is #{char}'
