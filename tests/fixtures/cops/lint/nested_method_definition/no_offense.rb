def foo
  something
end

def with_qualified_scope
  ::Class.new do
    def inner
      work
    end
  end
end

# Singleton method definitions with allowed receiver types

# def on local variable receiver
class Foo
  def x(obj)
    def obj.y
    end
  end
end

# def on instance variable receiver
class Foo
  def x
    def @obj.y
    end
  end
end

# def on class variable receiver
class Foo
  def x
    def @@obj.y
    end
  end
end

# def on global variable receiver
class Foo
  def x
    def $obj.y
    end
  end
end

# def on constant receiver
class Foo
  def x
    def Const.y
    end
  end
end

# def on method call receiver
class Foo
  def x
    def do_something.y
    end
  end
end

# def on parenthesized method call receiver
class Foo
  def x
    def (ActiveRecord::Base.connection).index_name_exists?(*)
      false
    end
  end
end

# def on parenthesized safe-navigation method call receiver
class Foo
  def x
    def (do_something&.y).z
    end
  end
end

# def on `it` block parameter receiver
def foo
  [1].each do
    def it.attached? = true
  end
end

# Scope-creating calls suppress offense
def foo
  self.class.class_eval do
    def bar
    end
  end
end

def foo
  mod.module_eval do
    def bar
    end
  end
end

def foo
  obj.instance_eval do
    def bar
    end
  end
end

def foo
  klass.class_exec do
    def bar
    end
  end
end

def foo
  mod.module_exec do
    def bar
    end
  end
end

def foo
  obj.instance_exec do
    def bar
    end
  end
end

# Class.new / Module.new / Struct.new blocks
def self.define
  Class.new do
    def y
    end
  end
end

def self.define
  Module.new do
    def y
    end
  end
end

def self.define
  Struct.new(:name) do
    def y
    end
  end
end

def self.define
  ::Struct.new do
    def y
    end
  end
end

# Data.define (Ruby 3.2+)
def self.define
  Data.define(:name) do
    def y
    end
  end
end

def self.define
  ::Data.define(:name) do
    def y
    end
  end
end

# class << self (singleton class) inside def
def bar
  class << self
    def baz
    end
  end
end

# define_method is a scope-creating call
def foo
  define_method(:bar) do
    def helper
    end
  end
end

# Nested def inside def inside class << self (scope-creating ancestor above outer def)
class NilNode
  class << self
    def instance
      @instance ||= begin
        def instance
          return @instance
        end
        new
      end
    end
  end
end

# Nested def inside def inside Struct.new block (scope-creating ancestor above outer def)
Control = Struct.new(:code, :desc) do
  def initialize(raw_data)
    self[:code] = raw_data[:code]

    def status
      :ok
    end
  end
end

# Nested def inside def inside Class.new block
klass = Class.new do
  def setup
    def helper
      42
    end
  end
end

# Nested def inside def inside Module.new block
mod = Module.new do
  def configure
    def validate
      true
    end
  end
end

# Nested def inside def inside define_method block
class Builder
  define_method(:build) do
    def assemble
      def connect
        true
      end
    end
  end
end
