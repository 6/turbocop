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

# Singleton class (class << self) should also be checked
class << self
  def some_method
    1
  end
  CONST = 1
  ^^^^^ Layout/ClassStructure: `constants` is supposed to appear before `public_methods`.
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

# Inline visibility declaration: `private :method_name` makes the def private
# for ordering purposes, so a subsequent public method triggers an offense.
class InlinePrivateDeclaration
  def validate_back_url(back_url)
    back_url
  end
  private :validate_back_url

  def redirect_to_referer_or
  ^^^ Layout/ClassStructure: `public_methods` is supposed to appear before `private_methods`.
  end
end

# Inline protected declaration: `protected :method_name` makes the def protected
class InlineProtectedDeclaration
  def scm_entries(path = nil)
    path
  end
  protected :scm_entries

  def entries(path = nil)
  ^^^ Layout/ClassStructure: `public_methods` is supposed to appear before `protected_methods`.
    path
  end
end

# Receiver-having call should still be classified (e.g. singleton_class.prepend)
class ReceiverCalls
  def some_method
    1
  end
  singleton_class.prepend SomeModule
  ^^^^^^^^^^^^^^^ Layout/ClassStructure: `module_inclusion` is supposed to appear before `public_methods`.
end
