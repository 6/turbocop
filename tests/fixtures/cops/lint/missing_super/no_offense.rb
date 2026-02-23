# No parent class - no super needed
class Child
  def initialize
    do_something
  end
end

# Calls super
class Child < Parent
  def initialize
    super
    do_something
  end
end

# Stateless parent Object
class Child < Object
  def initialize
    do_something
  end
end

# Stateless parent BasicObject
class Child < BasicObject
  def initialize
    do_something
  end
end

# Class.new without parent
Class.new do
  def initialize
    do_something
  end
end

# Class.new with stateless parent
Class.new(Object) do
  def initialize
    do_something
  end
end

# Module - not a class
module M
  def initialize
    do_something
  end
end

# Callback with super
class Foo
  def self.inherited(base)
    super
    do_something
  end
end

# method_added with super
class Foo
  def method_added(name)
    super
    do_something
  end
end
