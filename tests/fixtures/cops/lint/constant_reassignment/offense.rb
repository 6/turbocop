X = :foo
X = :bar
^ Lint/ConstantReassignment: Constant `X` is already assigned in this namespace.

Y = 1
Y = 2
^ Lint/ConstantReassignment: Constant `Y` is already assigned in this namespace.

Z = 'hello'
Z = 'world'
^ Lint/ConstantReassignment: Constant `Z` is already assigned in this namespace.
