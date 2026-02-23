class FooError < StandardError; end

class FooError < StandardError
end

Class.new(Settings::Base) do
  def repositories(*_args); end
end

local_var = Class.new(Base)
MyClass = Class.new(self)
