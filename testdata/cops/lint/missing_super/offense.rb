class Child < Parent
  def initialize
  ^^^^^^^^^^^^^^ Lint/MissingSuper: Call `super` to initialize state of the parent class.
  end
end

class Child < Parent
  def initialize(name, salary)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/MissingSuper: Call `super` to initialize state of the parent class.
    @salary = salary
  end
end

Class.new(Parent) do
  def initialize
  ^^^^^^^^^^^^^^ Lint/MissingSuper: Call `super` to initialize state of the parent class.
  end
end

class Foo
  def self.inherited(base)
  ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/MissingSuper: Call `super` to invoke callback defined in the parent class.
  end
end

class Foo
  def method_added(*)
  ^^^^^^^^^^^^^^^^^^^ Lint/MissingSuper: Call `super` to invoke callback defined in the parent class.
  end
end

class Foo
  class << self
    def inherited(base)
    ^^^^^^^^^^^^^^^^^^^ Lint/MissingSuper: Call `super` to invoke callback defined in the parent class.
    end
  end
end
