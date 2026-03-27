x == x
^^^^^^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `==` has identical operands.
a && a
^^^^^^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `&&` has identical operands.
b || b
^^^^^^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `||` has identical operands.
y >= y
^^^^^^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `>=` has identical operands.

:ruby == :"ruby"
^^^^^^^^^^^^^^^^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `==` has identical operands.

-0.0 <=> 0.0
^^^^^^^^^^^^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `<=>` has identical operands.

1.<(1, 2)
^^^^^^^^^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `<` has identical operands.

@tester.assert("\0REQ" == magic || "\000REQ" == magic)
               ^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `||` has identical operands.

arg_name == :_implicitBlockYield || arg_name == :"_implicitBlockYield"
^ Lint/BinaryOperatorWithIdenticalOperands: Binary operator `||` has identical operands.
