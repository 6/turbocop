class Foo
  def bar
    <<~SQL
      'Hi'
  SQL
  ^^^ Layout/ClosingHeredocIndentation: `SQL` is not aligned with `<<~SQL`.
  end
end

class Baz
  def qux
    <<~RUBY
      something
        RUBY
        ^^^^ Layout/ClosingHeredocIndentation: `RUBY` is not aligned with `<<~RUBY`.
  end
end

def example
  <<-TEXT
    hello
      TEXT
      ^^^^ Layout/ClosingHeredocIndentation: `TEXT` is not aligned with `<<-TEXT`.
end
