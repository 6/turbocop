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

@count +=
^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  items.size

@@total +=
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  items.size

$counter +=
^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  items.size

@cache ||=
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  compute_value

@flag &&=
^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  check_flag

# Multiline regex — RuboCop's safe_to_split? does not check :regexp,
# so assignments containing multiline regexps are still flaggable.
pattern = /
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  \A
  (?<key>.+)
  \z
/x

# Multiline %w array — RuboCop's safe_to_split? does not check arrays.
names = %w[
^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  alpha
  beta
  gamma
]

PWN::Plugins::BlackDuckBinaryAnalysis.abort_product_scan(
^ Layout/RedundantLineBreak: Redundant line break detected.

extract_uris(
^ Layout/RedundantLineBreak: Redundant line break detected.

cmd(
^ Layout/RedundantLineBreak: Redundant line break detected.

spec.add_development_dependency(
^ Layout/RedundantLineBreak: Redundant line break detected.

spec.add_dependency(
^ Layout/RedundantLineBreak: Redundant line break detected.

Trip::InternalError.new(
^ Layout/RedundantLineBreak: Redundant line break detected.

Trip::PauseError.new(
^ Layout/RedundantLineBreak: Redundant line break detected.
