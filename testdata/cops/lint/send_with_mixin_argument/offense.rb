Foo.send(:include, Bar)
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SendWithMixinArgument: Use `include Bar` instead of `send(:include, Bar)`.

Foo.public_send(:prepend, Bar)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SendWithMixinArgument: Use `prepend Bar` instead of `public_send(:prepend, Bar)`.

Foo.__send__(:extend, Bar)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SendWithMixinArgument: Use `extend Bar` instead of `__send__(:extend, Bar)`.
