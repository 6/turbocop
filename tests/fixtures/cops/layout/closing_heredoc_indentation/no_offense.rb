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
