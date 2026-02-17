if x
  y
end

class X
  y
end

begin
  x
end

def x
  y
end

module X
  y
end

class X # :nodoc:
  y
end

def x # rubocop:disable Metrics/MethodLength
  y
end

# module Y # trap comment

'end' # comment

def x(y = "#value")
  y
end

# end used as method receiver (method chain) with comment
end.to not_change(Conversation, :count) # No conversation created yet

# end followed by dot and method call
end.to eq(42) # some comment

# Comments inside heredocs are not real comments — parser doesn't see them
x = <<~RUBY
  class Foo # this is inside a heredoc
    def bar # also inside heredoc
    end # not a real comment
  end # still inside heredoc
RUBY
