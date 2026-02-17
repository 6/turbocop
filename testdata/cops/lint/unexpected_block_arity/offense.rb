values.reduce {}
^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 0.
values.reduce { |a| a }
^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `reduce` expects at least 2 positional arguments, got 1.
values.inject { |a| a }
^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnexpectedBlockArity: `inject` expects at least 2 positional arguments, got 1.
