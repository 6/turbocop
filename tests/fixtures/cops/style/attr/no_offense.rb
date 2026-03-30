class Foo
  attr_reader :something

  attr_accessor :name

  attr_writer :value
end

class SomeClass
  def attr(*args)
    args
  end

  def call
    attr(:name)
  end
end
