class Foo
  private

  def method
  end
end

class Bar
  protected

  def method2
  end
end

# MethodCreatingMethods: private followed by def_node_matcher
# This uses MethodCreatingMethods config which is not set in test defaults,
# but when configured properly, this should pass.
class Baz
  private

  def normal_method
  end
end

# define_method inside an each block — access modifier is not useless
class WithDefineMethodInIteration
  private

  [1, 2].each do |i|
    define_method("method#{i}") do
      i
    end
  end
end

# public after private, before a block that contains define_method
class WithDefineMethodInBlock
  private

  def some_private_method
  end

  public

  (CONFIGURABLE + NOT_CONFIGURABLE).each do |option|
    define_method(option) { @config[option] }
  end
end

# private before begin..end containing a method def
class WithBeginBlock
  private
  begin
    def method_in_begin
    end
  end
end

# private before lambda containing a def — not useless
class WithLambdaDef
  private

  -> {
    def some_method; end
  }.call
end

# private before proc containing a def — not useless
class WithProcDef
  private

  proc {
    def another_method; end
  }.call
end

# private_class_method with arguments is not useless
class WithPrivateClassMethodArgs
  private_class_method def self.secret
    42
  end
end

# private before private_class_method with args — not useless
# (matches RuboCop behavior where private_class_method with args
# resets access modifier tracking)
class WithPrivateBeforePrivateClassMethod
  private

  private_class_method def self.secret
    42
  end
end

# private before case with method definitions in branches — not useless
class WithCaseContainingDefs
  private

  case RUBY_ENGINE
  when "ruby"
    def get_result
      @result
    end
  when "jruby"
    def get_result
      @result
    end
  end
end

# FP fix: private inside class_eval block that is inside a def method
# RuboCop's macro? check means private is not recognized as access modifier here
module WithClassEvalInsideDef
  def self.define_class_methods(target)
    target.class_eval do
      define_singleton_method :update_data do |data|
        process(data)
      end

      private

      define_singleton_method :secret_data do
        fetch_secret
      end
    end
  end
end

# FP fix: public after conditional access modifier (protected unless $TESTING)
# visibility is changed by the conditional branch, so public is meaningful
class WithConditionalAccessModifier
  protected unless $TESTING

  SOME_CONSTANT = 42

  def some_method
    SOME_CONSTANT
  end

  attr_reader :name
  if $TESTING then
    attr_writer :name
    attr_accessor :data, :flags
  end

  public

  def initialize(name)
    @name = name
  end
end
