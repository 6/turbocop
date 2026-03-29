def foo
^^^^^^^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
  'bar'
end

def baz
^^^^^^^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
  42
end

def helper
^^^^^^^^^^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
  true
end

define_method(:foo) do |x|
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
  puts x
end

define_method(:foo, instance_method(:bar))
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.

XDR::Union.define_method(:method_missing) do |name, *args|
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
  return super(name, *args) unless value&.respond_to?(name)
  value&.public_send(name, *args)
end

XDR::Union.define_method(:respond_to_missing?) do |*args|
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
  value&.respond_to?(*args)
end

RSpec::Mocks::Syntax.singleton_class.define_method(:enable_should) { |*| nil }
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.

RSpec::Mocks::Syntax.singleton_class.define_method(:disable_should) { |*| nil }
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.

N1Loader::Loader.define_method :preloaded_records do
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
  @preloaded_records ||= loaded? && loaded_by_value.values.flatten
end

Onetime::CustomDomain.define_method(:generate_txt_validation_record, @original_method)
^ Style/TopLevelMethodDefinition: Do not define methods at the top level.
