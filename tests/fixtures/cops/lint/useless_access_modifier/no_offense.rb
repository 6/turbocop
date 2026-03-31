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

# FP fix: private after private_class_method with args is still meaningful
class WithPrivateAfterPrivateClassMethod
  def self.secret
    42
  end

  private_class_method :secret

  private

  def helper
    42
  end
end

# FP fix: repeated private_class_method declarations do not make a later private useless
class WithMultiplePrivateClassMethodsBeforePrivate
  def self.parse_container
    42
  end
  private_class_method :parse_container

  def self.parse_files
    42
  end
  private_class_method :parse_files

  private

  def add_item_internal
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

# FP fix: access modifier with chained method call (not a bare access modifier)
# e.g., module_function.should equal(nil) — module_function is the receiver of .should
Module.new do
  module_function.should equal(nil)
end

# FP fix: private/protected/public with chained method call
(class << Object.new; self; end).class_eval do
  def foo; end
  private.should equal(nil)
end

(class << Object.new; self; end).class_eval do
  def foo; end
  protected.should equal(nil)
end

(class << Object.new; self; end).class_eval do
  def foo; end
  public.should equal(nil)
end

# FP fix: private + def inside unrecognized block inside single-statement module body
# RuboCop's check_node only calls check_scope on begin-type bodies (multiple statements)
module WithPrivateInUnrecognizedBlock
  describe Hooks do
    build_hooked do
      before :add_around

      private

      def add_around
      end
    end
  end
end

# FP fix: module_function followed by inline access modifier (public def)
# `public def configure_maps` is a method definition decorated with an inline
# access modifier — module_function is not useless because it changed visibility.
module GeocoderHelpers
  def fill_in_geocoding(attribute, options = {})
    fill_in attribute, **options
  end

  module_function

  public def configure_maps
    Decidim.maps = { provider: :test }
  end
end

# FP fix: private followed by method decorator with def (memoize def)
# `memoize def entity` is a method definition — private is not useless.
class WithMemoizeDef
  def respond_to_missing?(name, *)
    entity.respond_to?(name)
  end

  private

  memoize def entity
    load
  end
end

# FP fix: public after private_class_method with args resets visibility tracking
# In RuboCop, private_class_method with args returns nil from check_send_node,
# setting cur_vis to nil (unknown). A subsequent public is a new change, not a repeat.
class WithPublicAfterPrivateClassMethodArgs
  def self.build_section(all_sections, name)
    all_sections
  end

  private

  def section_header_text(model)
    model
  end

  private_class_method :build_section

  def prepare_master_list
    @master_list = []
  end

  public

  IVS_TO_REMOVE = [:@records]

  def marshal_dump
    instance_variables
  end
end

# Inline access modifier with private def
class WithInlinePrivateDef
  protected

  private def secret_method
    42
  end
end

# Decorator followed by def in various patterns
class WithDecoratorDef
  private

  override def some_method
    super
  end
end
