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
