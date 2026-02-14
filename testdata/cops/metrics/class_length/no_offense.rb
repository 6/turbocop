class ShortClass
  def foo
    1
  end
end

class EmptyClass
end

class AnotherShort
  attr_reader :name
  attr_writer :age

  def initialize(name, age)
    @name = name
    @age = age
  end

  def greet
    "Hello, #{@name}"
  end
end
