it do
  foo = instance_double("Foo")
end
it do
  foo = class_double("Foo")
end
it do
  foo = object_double("Foo")
end
it do
  foo = double
end
it do
  foo = double(call: :bar)
end
