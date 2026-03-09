my_method(1) \
^^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  [:a]

foo && \
^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  bar

foo || \
^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  bar

my_method(1,
^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
          2,
          "x")

foo(' .x')
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  .bar
  .baz

a =
^^^ Layout/RedundantLineBreak: Redundant line break detected.
  m(1 +
    2 +
    3)

b = m(4 +
^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
      5 +
      6)

raise ArgumentError,
^^^^^^^^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
      "can't inherit configuration from the rubocop gem"

foo(x,
^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
    y,
    z)
  .bar
  .baz

x = [
^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  1,
  2,
  3
]

y = {
^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  a: 1,
  b: 2
}

foo(
^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  bar(1, 2)
)
