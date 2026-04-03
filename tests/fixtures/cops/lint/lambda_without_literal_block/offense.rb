lambda(&proc { do_something })
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
lambda(&Proc.new { do_something })
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
lambda(&pr)
^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
describe lambda('my-func') do
         ^^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
  it { should exist }
end
@parent.lambda(name, &block)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
lambda(some_var)
^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.

result = lines.flat_map(&split).reduce(&lambda(&method(:longest_words)))
                                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.

foo do |b|
  lambda(&b).call
  ^^^^^^^^^^ Lint/LambdaWithoutLiteralBlock: lambda without a literal block is deprecated; use the proc without lambda instead.
end
