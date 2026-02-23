refine Foo do
  import_methods Bar
end

class MyClass
  include Bar
end

module MyModule
  prepend Baz
end
