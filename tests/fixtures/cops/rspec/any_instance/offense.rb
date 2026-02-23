allow_any_instance_of(Object).to receive(:foo)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/AnyInstance: Avoid stubbing using `allow_any_instance_of`.
expect_any_instance_of(Object).to receive(:foo)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/AnyInstance: Avoid stubbing using `expect_any_instance_of`.
Object.any_instance.should_receive(:foo)
^^^^^^^^^^^^^^^^^^^ RSpec/AnyInstance: Avoid stubbing using `any_instance`.
