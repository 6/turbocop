{a: 1, b: 2}

{a: 1}

{}

{x: "hello", y: "world"}

{foo: :bar}

# Comma inside a comment between last element and closing brace
{
  'name' => 'hello'
  # No language, should default
}

# Heredoc value whose body contains a comma — not a trailing comma
{
  key: <<RUBY
hello, world
RUBY
}

# Heredoc in array value with comma in body
{
  key: [method(<<RUBY)]
foo(a, b)
RUBY
}

# Heredoc with delete(',') pattern — comma in string, not trailing
{
  foo: 'foo',
  bar: 'bar'.delete(',')
}

# Heredoc method call with commas in the body is not a trailing hash comma
{
  key: <<~YAML.unindent
    one, two
  YAML
}

# Nested hash containing a heredoc should not make the outer hash look like it
# has a trailing comma when the heredoc body contains commas.
modules = {
  'mod' => { 'lib' => { 'uri_test_func.rb' => <<-RUBY } }
    def uri_test_func(options, context)
      { 'uri' => [options['uri']] }
    end
  RUBY
}
