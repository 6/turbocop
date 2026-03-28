class Foo
  public
  ^^^^^^ Lint/UselessAccessModifier: Useless `public` access modifier.

  def method
  end
end

class Bar
  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.
end

class Baz
  protected
  ^^^^^^^^^ Lint/UselessAccessModifier: Useless `protected` access modifier.
end

module Qux
  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  def self.singleton_method
  end
end

# private_class_method without arguments is useless
class WithPrivateClassMethod
  private_class_method
  ^^^^^^^^^^^^^^^^^^^^ Lint/UselessAccessModifier: Useless `private_class_method` access modifier.

  def self.calculate_something(data)
    data
  end
end

# top-level access modifiers are always useless
private
^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

def top_level_method
end

protected
^^^^^^^^^ Lint/UselessAccessModifier: Useless `protected` access modifier.

def another_top_level_method
end

# module_function at top level is useless
module_function
^^^^^^^^^^^^^^^ Lint/UselessAccessModifier: Useless `module_function` access modifier.

def top_func
end

# module_function inside a module followed only by eval is useless
module WithModuleFunction
  module_function
  ^^^^^^^^^^^^^^^ Lint/UselessAccessModifier: Useless `module_function` access modifier.
  eval "def test1() end"
end

# module_function repeated inside a module
module RepeatedModuleFunction
  module_function

  def first_func; end

  module_function
  ^^^^^^^^^^^^^^^ Lint/UselessAccessModifier: Useless `module_function` access modifier.

  def second_func; end
end

# useless access modifier inside Class.new do block
Class.new do
  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.
end

# FN fix: private repeated due to visibility leaking from conditional branch
# RuboCop's check_child_nodes recurses into if/else, propagating cur_vis
class WithVisibilityFromConditional
  if some_condition
    private

    def secret_method
    end
  end

  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  def another_method
  end
end

# FN fix: private before class definition + method def, where visibility
# leaked from inside a block making it a repeated modifier
class WithBlockVisibilityLeak
  some_dsl :items do
    property :title

    private

    def populate_item!
      Item.new
    end
  end

  property :artist do
    property :name
  end

  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  class Helper < Base
    attr_accessor :args
  end

  def create_item(input)
    Helper.new
  end
end

# FN fix: included do is still analyzed when the surrounding module body has
# multiple statements, matching the corpus ActiveSupport::Concern pattern
module WithIncludedSingletonMethod
  extend ActiveSupport::Concern

  included do
    private
    ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

    def self.singleton_method_added(method_name)
      method_name
    end
  end
end

# FN fix: prior singleton defs inside an included block do not make a later
# private meaningful when the block is reached through the enclosing module body
module WithIncludedSingletonMethodsAroundPrivate
  SOME_CONSTANT = 42

  included do
    def self.method_missing(name, *)
      name
    end

    private
    ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

    def self.all_types
      []
    end
  end
end

# FN fix: private after a singleton def in a class body is still useless
class WithSingletonDefs
  def self.page(page)
    page
  end

  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  def self.base_options(options)
    options
  end
end

# FN fix: private after an instance method but before only singleton defs is useless
module WithSingletonDefAfterInstanceMethod
  def helper
    42
  end

  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  def self.create_related_elements(doc)
    doc
  end
end

# FN fix: singleton defs written as one-liners still do not use private visibility
class WithOneLineSingletonDef
  def self.variants; constants; end

  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  def self.guard_context(obj)
    obj
  end
end
