foo = instance_double(Foo)
bar = instance_spy(Bar)
baz = instance_double(Baz).tap { |x| x }
qux = double(Qux).as_null_object
spy = instance_spy(Foo).as_null_object

# instance_double.as_null_object used purely as a stub (no message expectations)
let(:config) { instance_double(RuboCop::Config, loaded_path: '.rubocop.yml').as_null_object }
