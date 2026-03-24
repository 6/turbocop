class MyClass
  def foo
  ^^^ Style/TrivialAccessors: Use `attr_reader` to define trivial reader methods.
    @foo
  end

  def bar
  ^^^ Style/TrivialAccessors: Use `attr_reader` to define trivial reader methods.
    @bar
  end

  def baz=(val)
  ^^^ Style/TrivialAccessors: Use `attr_writer` to define trivial writer methods.
    @baz = val
  end

  # class methods (def self.foo) should be flagged by default
  def self.config
  ^^^ Style/TrivialAccessors: Use `attr_reader` to define trivial reader methods.
    @config
  end

  def self.config=(val)
  ^^^ Style/TrivialAccessors: Use `attr_writer` to define trivial writer methods.
    @config = val
  end
end

# Methods inside blocks (describe, Class.new, etc.) should be flagged
describe "something" do
  def app
  ^^^ Style/TrivialAccessors: Use `attr_reader` to define trivial reader methods.
    @app
  end
end

# Methods inside nested blocks should be flagged
describe "outer" do
  context "inner" do
    def name
    ^^^ Style/TrivialAccessors: Use `attr_reader` to define trivial reader methods.
      @name
    end
  end
end

# Singleton methods on objects inside blocks should be flagged
describe "test" do
  obj = Object.new
  def obj.status
  ^^^ Style/TrivialAccessors: Use `attr_reader` to define trivial reader methods.
    @status
  end
end
