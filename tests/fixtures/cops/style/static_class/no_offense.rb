module Foo
  def self.bar
    42
  end
end

class Bar
  def instance_method
    'hello'
  end
end

class Child < Parent
  def self.class_method
    42
  end
end

class WithInclude
  include SomeModule
  def self.class_method
    42
  end
end

class WithPrepend
  prepend SomeModule
  def self.class_method
    42
  end
end

class WithMacroCall
  def self.class_method; end
  macro_method
end

class WithSclassPrivate
  class << self
    def public_method; end

    private

    def private_method; end
  end
end

class WithSclassMacro
  def self.class_method; end

  class << self
    attr_accessor :setting
  end
end

class Empty
end

class WithPrivateClassMethod
  def self.public_method; end
  private_class_method :public_method
end
