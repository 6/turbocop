x == 0.1
^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x != 0.1
^^^^^^^^ Lint/FloatComparison: Avoid inequality comparisons of floats as they are unreliable.
0.5 == y
^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x.to_f == 1
^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
1 == x.to_f
^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
n.to_f != 0.1
^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid inequality comparisons of floats as they are unreliable.
x.fdiv(2) == 1
^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
Float(x) == 1
^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.

case value
when 1.0
     ^^^ Lint/FloatComparison: Avoid float literal comparisons in case statements as they are unreliable.
  foo
when 2.0
     ^^^ Lint/FloatComparison: Avoid float literal comparisons in case statements as they are unreliable.
  bar
end

case value
when 1.0, 2.0
          ^^^ Lint/FloatComparison: Avoid float literal comparisons in case statements as they are unreliable.
     ^^^ Lint/FloatComparison: Avoid float literal comparisons in case statements as they are unreliable.
  foo
end

x == 0.1 + y
^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x == y + Float('0.1')
^^^^^^^^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x == 2.0 ** -52
^^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x == 0.1.abs
^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x == 0.0.next_float
^^^^^^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x == 0.0.prev_float
^^^^^^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x == 1.1.ceil(1)
^^^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
n.to_f % 10 == 1
^^^^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
x == 280.0 / 355.0
^^^^^^^^^^^^^^^^^^^ Lint/FloatComparison: Avoid equality comparisons of floats as they are unreliable.
