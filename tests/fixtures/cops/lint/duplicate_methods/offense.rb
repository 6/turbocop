# Basic duplicate instance method in class
class Foo
  def bar
    1
  end

  def bar
  ^^^^^^^ Lint/DuplicateMethods: Method `Foo#bar` is defined at both test.rb:3 and test.rb:7.
    2
  end
end

# Duplicate in module
module MyMod
  def helper
    true
  end

  def helper
  ^^^^^^^^^^ Lint/DuplicateMethods: Method `MyMod#helper` is defined at both test.rb:14 and test.rb:18.
    false
  end
end

# Duplicate self method
class Widget
  def self.create
    1
  end

  def self.create
  ^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `Widget.create` is defined at both test.rb:25 and test.rb:29.
    2
  end
end

# Duplicate alias
class WithAlias
  def render
    1
  end
  alias render other
  ^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `WithAlias#render` is defined at both test.rb:36 and test.rb:39.
end

# Duplicate alias_method
class WithAliasMethod
  def process
    1
  end
  alias_method :process, :other
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `WithAliasMethod#process` is defined at both test.rb:44 and test.rb:47.
end

# Duplicate attr_reader
class WithAttr
  def value
  end
  attr_reader :value
  ^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `WithAttr#value` is defined at both test.rb:52 and test.rb:54.
end

# Duplicate attr_writer
class WithAttrWriter
  def value=(right)
  end
  attr_writer :value
  ^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `WithAttrWriter#value=` is defined at both test.rb:59 and test.rb:61.
end

# Duplicate attr_accessor (both reader and writer)
class WithAttrAccessor
  attr_accessor :data

  def data
  ^^^^^^^^ Lint/DuplicateMethods: Method `WithAttrAccessor#data` is defined at both test.rb:66 and test.rb:68.
  end
  def data=(right)
  ^^^^^^^^^ Lint/DuplicateMethods: Method `WithAttrAccessor#data=` is defined at both test.rb:66 and test.rb:70.
  end
end

# private def (inline modifier)
class WithPrivate
  private def compute
    1
  end
  private def compute
          ^^^^^^^^^^^ Lint/DuplicateMethods: Method `WithPrivate#compute` is defined at both test.rb:76 and test.rb:79.
    2
  end
end

# Top-level duplicate methods
def some_method
  1
end
def some_method
^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `Object#some_method` is defined at both test.rb:85 and test.rb:88.
  2
end

# Reopened class
class Reopened
  def act
    1
  end
end
class Reopened
  def act
  ^^^^^^^ Lint/DuplicateMethods: Method `Reopened#act` is defined at both test.rb:94 and test.rb:99.
    2
  end
end

# class << self
class Singleton
  class << self
    def call
      1
    end
    def call
    ^^^^^^^^ Lint/DuplicateMethods: Method `Singleton.call` is defined at both test.rb:107 and test.rb:110.
      2
    end
  end
end

# Nested modules
module Outer
  class Inner
    def process
      1
    end
    def process
    ^^^^^^^^^^^ Lint/DuplicateMethods: Method `Outer::Inner#process` is defined at both test.rb:119 and test.rb:122.
      2
    end
  end
end

# def_delegator
class WithDelegator
  def_delegator :target, :action

  def action; end
  ^^^^^^^^^^ Lint/DuplicateMethods: Method `WithDelegator#action` is defined at both test.rb:130 and test.rb:132.
end

# def_delegators
class WithDelegators
  def_delegators :target, :run, :stop

  def run; end
  ^^^^^^^ Lint/DuplicateMethods: Method `WithDelegators#run` is defined at both test.rb:137 and test.rb:139.
end

# def ConstName.method resolving to outer scope
module Container
  class Child
    def Container.helper; 1; end
    def Container.helper; 2; end
    ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `Container.helper` is defined at both test.rb:145 and test.rb:146.
  end
end

# def A.method and def self.method should be same
class Unified
  def Unified.compute; 1; end
  def self.compute; 2; end
  ^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `Unified.compute` is defined at both test.rb:152 and test.rb:153.
end

# Reopened class << ConstName should detect duplicates
module Singleton
  def self.append_features(mod); 1; end
end
class << Singleton
  def append_features(mod); 2; end
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `Singleton.append_features` is defined at both test.rb:158 and test.rb:161.
end

# Reopened class << ConstName (two separate blocks)
class << Singleton
  def included(klass); 1; end
end
class << Singleton
  def included(klass); 2; end
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `Singleton.included` is defined at both test.rb:166 and test.rb:169.
end

# class << ConstName with attr_reader duplicating def
class << Singleton
  def count; 1; end
  attr_reader :count
  ^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `Singleton.count` is defined at both test.rb:174 and test.rb:175.
end

# case/when does NOT suppress duplicate detection (only if/unless does)
class CaseVariant
  case RUBY_VERSION
  when '3.0'
    def bar; 1; end
  when '2.7'
    def bar; 2; end
    ^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `CaseVariant#bar` is defined at both test.rb:182 and test.rb:184.
  end
end

# class << Object.new — send-type sclass expression
class << Object.new
  def meth; 1; end
  def meth; 2; end
  ^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `new.meth` is defined at both test.rb:190 and test.rb:191.
end

# class << some_call.chain — method name from outermost call
record = Object.new
class << record.response
  def body; 1; end
  def body; 2; end
  ^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `response.body` is defined at both test.rb:197 and test.rb:198.
end

# def ConstName.method where constant is NOT in scope (no class/module ancestor)
# RuboCop's lookup_constant returns the node itself when no ancestor matches,
# producing a key based on the full AST dump. Two identical defs match.
def FakeModel.calling_let!(*_args); end
def FakeModel.calling_let!(*_args); end
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `FakeModel.calling_let!` is defined at both test.rb:204 and test.rb:205.

# def ConstName.method at top level — same constant, same body
def VCR.version
  "2.0.0"
end
def VCR.version
^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `VCR.version` is defined at both test.rb:208 and test.rb:211.
  "2.0.0"
end

# Reopened class << @ivar.call — two separate sclass blocks with same send-type expression
class << record.response
  def content_type; 1; end
end
class << record.response
  def content_type; 2; end
  ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `response.content_type` is defined at both test.rb:217 and test.rb:220.
end

# def inside sclass expression (pry-style): Class.new block within sclass expr
# RuboCop's found_sclass_method catches defs inside the expression via ancestor traversal
class << Object.new
  def pry_meth; 1; end
end
class << Class.new {
  def pry_meth; 1; end
  ^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateMethods: Method `new.pry_meth` is defined at both test.rb:226 and test.rb:229.
}.new
  def placeholder; end
end
