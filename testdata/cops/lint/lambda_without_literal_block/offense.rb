lambda(&proc { do_something })
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
lambda(&Proc.new { do_something })
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
lambda(&pr)
^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
