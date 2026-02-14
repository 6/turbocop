foo = instance_double(Foo).as_null_object
      ^^^^^^^^^^^^^^^^^^^^ RSpec/InstanceSpy: Use `instance_spy` when you check your double with `have_received`.
bar = instance_double(Bar).as_null_object
      ^^^^^^^^^^^^^^^^^^^^ RSpec/InstanceSpy: Use `instance_spy` when you check your double with `have_received`.
baz = instance_double(Baz, :name).as_null_object
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/InstanceSpy: Use `instance_spy` when you check your double with `have_received`.
