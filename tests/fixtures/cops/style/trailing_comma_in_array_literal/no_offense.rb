[1, 2, 3]

[1]

[]

["a", "b"]

[:foo, :bar]

# Word/symbol arrays don't use commas — never flagged
%w(
  foo
  bar
)

%i(foo bar baz)

%W[one two three]

# Multiline array with single element — closing bracket on same line as element
# (allowed_multiline_argument pattern — not flagged under no_comma default)
[
  some_method_call(
    arg1, arg2
  )]

# Multiline array that already has no trailing comma (no_comma style)
[
  1,
  2,
  3
]

# Array containing a heredoc element (heredoc content has commas, not array commas)
[
  <<~OUTPUT.chomp
    The `Style/PredicateName` cop has been renamed, please update it
  OUTPUT
]

# Single-line array with heredoc element — no trailing comma
cmd = ['-W0', '-e', <<-RB]
  puts 'foo'
  print 'bar'
RB

# Single-line array with multiple heredoc elements
x = [<<EOS1, <<EOS2]
first content
EOS1
second content
EOS2

# Heredoc content with comma-like text (FP fix — zeitwerk pattern)
# The heredoc content and terminator should not be confused with array commas
[
  "foo.rb",
  <<-EOS
    some content,
    more content,
  EOS
]

# Heredoc with CSS/SASS content containing commas (FP fix — thredded pattern)
[
  "header",
  <<~SASS
    .messageboard,
    .topic {
      color: red;
    }
  SASS
]

# Heredoc delimiter that includes special chars (FP fix — rufo pattern)
[<<~'},']
hello
},

# Nested array with heredoc pairs — zeitwerk pattern (FP fix)
# The last sub-array has no trailing comma; heredoc content should not be
# confused with an array-level comma.
files = [
  ["foo.rb", <<-EOS],
    require 'bar'
    Foo = 1
  EOS
  ["bar.rb", <<-EOS]
    Bar = true
    Object.autoload(:Foo, File.expand_path('foo.rb'))
    $trc_inception = !Object.autoload?(:Foo)
  EOS
]

# Nested array with heredoc — last element is sub-array ending with heredoc
x = [
  ["a.rb", <<~RUBY],
    class A; end
  RUBY
  ["b.rb", <<~RUBY]
    class B; end
  RUBY
]
