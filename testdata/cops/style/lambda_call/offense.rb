l = ->(x) { x }
l.(1)
^^^^^ Style/LambdaCall: Prefer the use of `lambda.call(...)` over `lambda.(...)`.

foo.(x, y)
^^^^^^^^^^ Style/LambdaCall: Prefer the use of `lambda.call(...)` over `lambda.(...)`.

bar.()
^^^^^^ Style/LambdaCall: Prefer the use of `lambda.call(...)` over `lambda.(...)`.
