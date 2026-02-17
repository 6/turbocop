some_method a { |el| puts el }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousBlockAssociation: Parenthesize the param `a { |el| puts el }` to make sure that the block will be associated with the `a` method call.
Foo.some_method a { |el| puts el }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousBlockAssociation: Parenthesize the param `a { |el| puts el }` to make sure that the block will be associated with the `a` method call.
expect { order.expire }.to change { order.events }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousBlockAssociation: Parenthesize the param `change { order.events }` to make sure that the block will be associated with the `change` method call.
