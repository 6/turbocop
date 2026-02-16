def foo
  @bar
  @foo
end

def baz?
  @baz
end

attr_reader :name

attr_writer :age

def complex
  @value + 1
end
