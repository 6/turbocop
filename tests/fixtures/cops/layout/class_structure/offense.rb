class Foo
  def bar
    1
  end
  include Comparable
  ^^^^^^^ Layout/ClassStructure: `module_inclusion` is supposed to appear before `public_methods`.
end

class Baz
  def initialize
    @x = 1
  end
  CONST = 1
  ^^^^^ Layout/ClassStructure: `constants` is supposed to appear before `initializer`.
end

class Qux
  def qux_method
    2
  end
  include Enumerable
  ^^^^^^^ Layout/ClassStructure: `module_inclusion` is supposed to appear before `public_methods`.
end

# Only the FIRST out-of-order element triggers, not subsequent same-category ones
class CascadeTest
  CONST = 1
  def instance_method
    2
  end
  def self.class_method_a
  ^^^ Layout/ClassStructure: `public_class_methods` is supposed to appear before `public_methods`.
  end
  def self.class_method_b
  end
end

# Multiple includes after constant: only the first triggers
class IncludeCascade
  CONST = 1
  include Comparable
  ^^^^^^^ Layout/ClassStructure: `module_inclusion` is supposed to appear before `constants`.
  include Enumerable
  include Kernel
end

# Protected after private: only the first triggers
class VisibilityCascade
  private

  def private_method
    1
  end

  protected

  def first_protected
  ^^^ Layout/ClassStructure: `protected_methods` is supposed to appear before `private_methods`.
  end
  def second_protected
  end
end
