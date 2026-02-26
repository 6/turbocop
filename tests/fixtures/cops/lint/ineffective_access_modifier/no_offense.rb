class C
  class << self
    private

    def method
      puts 'hi'
    end
  end
end

class D
  def self.method
    puts 'hi'
  end

  private_class_method :method
end

# private + def self.method + private_class_method :method (FP fix)
class PrivateBeforeClassMethod
  private

  def self.underscore(word)
    word.downcase
  end

  private_class_method :underscore
end

# private + multiple class methods + private_class_method with multiple args
class MultiplePrivateClassMethods
  private

  def self.run_core(fragment)
    fragment
  end

  def self.run_plugin(fragment)
    fragment
  end

  private_class_method :run_core, :run_plugin
end

# protected + def self.method + protected_class_method
class ProtectedClassMethodExample
  protected

  def self.helper_method
    'helper'
  end

  protected_class_method :helper_method
end

# private_class_method only covers some methods - uncovered ones still offend
class PartialCoverage
  def self.public_method
    'public'
  end
end
