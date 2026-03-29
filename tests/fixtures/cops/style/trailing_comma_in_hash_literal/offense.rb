{a: 1, b: 2,}
           ^ Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.

{x: "hello", y: "world",}
                       ^ Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.

{foo: 1,}
       ^ Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.

# Heredoc value with trailing comma (FN fix)
# nitrocop-expect: 9:26 Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.
example = {
  :mock_userinfo => <<~EOS,
    hello
  EOS
}

# Another squiggly heredoc value with trailing comma
# nitrocop-expect: 16:17 Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.
response = {
  :html => <<~EOS,
    <html></html>
  EOS
}

# Single-quoted heredoc delimiter as last hash value
# nitrocop-expect: 23:25 Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.
settings = {
  :desc       => <<-'EOT',
    docs
  EOT
}

# String key with plain heredoc
# nitrocop-expect: 30:24 Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.
files = {
  'init.pp' => <<-PUPPET,
    notify { 'hello': }
  PUPPET
}

# Method call on heredoc as last hash value
# nitrocop-expect: 37:34 Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.
config = {
  'hiera.yaml' => <<-YAML.unindent,
    ---
  YAML
}

# Another plain heredoc variant
# nitrocop-expect: 44:23 Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.
scripts = {
  'test3.rb' => <<-RUBY,
    puts :ok
  RUBY
}

# Another method call on heredoc variant
# nitrocop-expect: 51:33 Style/TrailingCommaInHashLiteral: Avoid comma after the last item of a hash.
types = {
  'mytest.rb' => <<-RUBY.unindent,
    puts :ok
  RUBY
}
