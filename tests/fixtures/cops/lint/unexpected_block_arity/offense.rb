values.reduce {}
^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 0.
values.reduce { |a| a }
^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 1.
values.inject { |a| a }
^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `inject` expects at least 2 positional arguments, got 1.
values.reduce { _1 }
^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 1.
values.reduce { it }
^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 1.
values.reduce { |a:, b:| a + b }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 0.
values.reduce { |**kwargs| kwargs }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 0.
values.reduce { |(a, b)| a + b }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 1.
values.reduce { |a; b| a + b }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 1.
values.each_with_index { |elem| elem }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `each_with_index` expects at least 2 positional arguments, got 1.
values.each_with_object([]) { |v| v }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `each_with_object` expects at least 2 positional arguments, got 1.
values.sort { |a| a }
^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `sort` expects at least 2 positional arguments, got 1.
values.chunk_while { |a| a }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `chunk_while` expects at least 2 positional arguments, got 1.
values.slice_when { |a| a }
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `slice_when` expects at least 2 positional arguments, got 1.
values.max { |a| a }
^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `max` expects at least 2 positional arguments, got 1.
values.min { |a| a }
^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `min` expects at least 2 positional arguments, got 1.
values.minmax { |a| a }
^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `minmax` expects at least 2 positional arguments, got 1.
