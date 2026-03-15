some_method a { |el| puts el }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousBlockAssociation: Parenthesize the param `a { |el| puts el }` to make sure that the block will be associated with the `a` method call.
Foo.some_method a { |el| puts el }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousBlockAssociation: Parenthesize the param `a { |el| puts el }` to make sure that the block will be associated with the `a` method call.
expect { order.expire }.to change { order.events }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousBlockAssociation: Parenthesize the param `change { order.events }` to make sure that the block will be associated with the `change` method call.
wrapper.call token, proc{!defined? _1.to_s} do |value|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousBlockAssociation: Parenthesize the param `proc{!defined? _1.to_s}` to make sure that the block will be associated with the `proc` method call.
end
