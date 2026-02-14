foo = instance_double(Foo)
bar = instance_spy(Bar)
baz = instance_double(Baz).tap { |x| x }
qux = double(Qux).as_null_object
spy = instance_spy(Foo).as_null_object
