class Foo
  def bar
    <<~SQL
      'Hi'
    SQL
  end
end

class Baz
  def qux
    <<~RUBY
      something
    RUBY
  end
end

def example
  <<-TEXT
    hello
  TEXT
end

x = <<SIMPLE
no indent required
SIMPLE

# Heredoc argument aligned to outermost call
include_examples :offense,
                 <<-HEREDOC
  bar
HEREDOC

# Heredoc argument with strip_indent aligned to outermost call
include_examples :offense,
                 <<-HEREDOC.strip_indent
  bar
HEREDOC

# Heredoc in block body, properly aligned
get '/foo' do
  <<-EOHTML
  <html></html>
  EOHTML
end

# Chained call with heredoc argument aligned to outermost call
expect($stdout.string)
  .to eq(<<~RESULT)
    content here
RESULT

# Nested call argument still aligns to the outermost call chain
foo bar(
  <<-EOF,
  baz
EOF
)

# Empty heredoc content, aligned
let(:source) { <<~HEREDOC }
HEREDOC
