# Different methods in a class
class Foo
  def bar
    1
  end

  def baz
    2
  end
end

# Same name in different classes
class Alpha
  def run
    :a
  end
end
class Beta
  def run
    :b
  end
end

# Instance and singleton methods with the same name are different
class Separate
  def foo
    :instance
  end

  def self.foo
    :class
  end
end

# Conditional method definitions should not be flagged
class Platform
  if RUBY_VERSION >= "3.0"
    def bar
      :modern
    end
  else
    def bar
      :legacy
    end
  end
end

# alias to self is allowed
class WithAlias
  alias foo foo
  def foo
    1
  end
end

# alias_method to self is allowed
class WithAliasMethod
  alias_method :foo, :foo
  def foo
    1
  end
end

# Non-duplicate alias (alias a different name)
class NonDupAlias
  def process
    1
  end
  alias other process
end

# Non-duplicate alias_method
class NonDupAliasMethod
  def process
    1
  end
  alias_method :other, :process
end

# attr_reader and setter are different
class AttrMismatch
  def value=(right)
  end
  attr_reader :value
end

# attr_writer and getter are different
class AttrMismatch2
  def value
  end
  attr_writer :value
end

# Same method name in different nested methods (scoped differently)
class Scoped
  def foo
    def inner
      1
    end
  end

  def bar
    def inner
      2
    end
  end
end

# Same method in different blocks
class BlockScoped
  dsl_like('foo') do
    def process
      1
    end
  end

  dsl_like('bar') do
    def process
      2
    end
  end
end

# alias_method with dynamic original name (not a symbol)
class DynamicAlias
  alias_method :process, unknown()
  def process
    1
  end
end

# alias_method inside condition
class ConditionalAlias
  def process
    1
  end

  if some_condition
    alias_method :process, :other
  end
end

# alias for global variables (not method alias)
class WithGvar
  alias $foo $bar
end

# def inside rescue/ensure scope reset
module Rescue
  def make_fail
    Klass.class_eval do
      def failed
        raise
      end
      alias_method :original, :save
      alias_method :save, :failed
    end

    yield
  ensure
    Klass.class_eval do
      alias_method :save, :original
    end
  end
end

# RSpec describe blocks are ignored (method scope is not class/module)
describe "something" do
  def helper
    1
  end
  def helper
    2
  end
end

# Class.new assigned to local variables are different scopes
a = Class.new do
  def foo
  end
end
b = Class.new do
  def foo
  end
end

# def_delegator inside condition
class ConditionalDelegator
  def_delegator :target, :action if some_condition?

  def action; end
end

# delegate without ActiveSupportExtensionsEnabled (default false)
class WithDelegate
  def process
    1
  end
  delegate :process, to: :bar
end

# Struct.new blocks are not recognized as scope by RuboCop (only Class/Module)
# Duplicates inside are ignored since parent_module_name returns nil for blocks
Alpha = Struct.new(:x) do
  def call; 1; end
  def call; 2; end
end

# Local Struct.new also not a scope
a = Struct.new(:x) do
  def call; 1; end
end
b = Struct.new(:y) do
  def call; 2; end
end

# module_eval is not recognized as scope-creating by RuboCop (only class_eval)
Klass.module_eval do
  def helper; 1; end
  def helper; 2; end
end

# implicit class_eval (no receiver) inside module - different methods are fine
module TransparentClassEval
  class_eval do
    def helper; 1; end
  end
  def other_helper; 1; end
end

# self.alias_method should be ignored (RuboCop only matches nil receiver)
alias_method :foo, :bar
self.alias_method :foo, :baz

# self.attr_reader / self.attr_writer / self.attr_accessor should be ignored
attr_reader :item
self.attr_reader :item

attr_writer :record
self.attr_writer :record

attr_accessor :entry
self.attr_accessor :entry

# Method calls on objects named attr should not match Ruby's attr method
class Parser
  def extract
    doc.attr('content')
    doc.attr('content')
    doc.attr('content')
  end
end

# alias_method with string args after alias_method with symbol args
# RuboCop's alias_method? pattern only matches symbol arguments, not strings
class AliasMethodStrings
  alias_method :process, :other
  alias_method "process", "other"
end

# delegate with ActiveSupportExtensionsEnabled — different methods, no conflict
class WithDelegateNoConflict
  delegate :name, to: :target
  def status; end
end

# def_delegators with non-symbol/string first arg should be ignored
class WithConstDelegators
  extend Forwardable
  def_delegators SomeModule, :run, :stop

  def run; end
end

# Nested class inside class << ConstName does NOT conflict with
# the same class inside module ConstName > class << self.
# RuboCop produces different scope keys for these two contexts.
class << Multiton
  class InstanceMutex
    def initialize; @m = Mutex.new; end
  end
end
module Multiton
  class << self
    class InstanceMutex
      def initialize; @m = Mutex.new; end
    end
  end
end

# Different methods in class << ConstName are fine
class << Multiton
  def foo; 1; end
  def bar; 2; end
end

# Different methods in class << call_expr are fine
class << Object.new
  def foo; 1; end
  def bar; 2; end
end

# def ConstName.method with different bodies — NOT detected by RuboCop
# because lookup_constant returns the full AST node (including body) as key
def VCR.version
  "1.0"
end
def VCR.version
  "2.0"
end
