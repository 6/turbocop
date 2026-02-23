class MyClass
  # multi-statement body is not trivial
  def foo
    @bar
    @foo
  end

  # predicate methods are allowed by default
  def baz?
    @baz
  end

  attr_reader :name

  attr_writer :age

  # body with expression is not trivial
  def complex
    @value + 1
  end

  # AllowedMethods: initialize is always allowed
  def initialize
    @name
  end

  # AllowedMethods: to_s, to_i, to_h, etc. are allowed by default
  def to_s
    @value
  end

  def to_i
    @number
  end

  def to_h
    @hash
  end

  def to_a
    @array
  end

  def to_proc
    @proc
  end

  def to_str
    @str
  end
end

# Methods inside modules are skipped (vendor's in_module_or_instance_eval? check)
module MyModule
  def name
    @name
  end

  def name=(val)
    @name = val
  end
end

# Top-level methods are also skipped
def top_level_reader
  @top_level_reader
end
