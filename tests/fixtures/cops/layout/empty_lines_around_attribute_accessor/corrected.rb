class Foo
  attr_accessor :foo

  def do_something
  end
end

class Bar
  attr_reader :bar

  def another_method
  end
end

class Baz
  attr_writer :baz

  def yet_another
  end
end

# attr_accessor followed by YARD comments then blank line then code — offense
# RuboCop flags because no blank line directly after the attr_accessor
class TensorOutput
  attr_accessor :index, :operation

  # @!attribute index
  # Index specifies the index of the output.
  # @!attribute operation
  # Operation is the Operation that produces this Output.

  def compute
  end
end

# attr_accessor followed by comments then blank line — offense
class SessionConfig
  attr_accessor :status, :graph

  # @!attribute dimensions
  # Dimensions of the graph.

  def run
  end
end

# attr_reader followed by single comment then code — offense
class CommentThenCode
  attr_reader :value

  # some comment
  def process
  end
end

# attr_writer followed by multiple comments then code — offense
class MultiCommentThenCode
  attr_writer :data

  # comment one
  # comment two
  def transform
  end
end
