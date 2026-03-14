X = :foo
X = :bar
^ Lint/ConstantReassignment: Constant `X` is already assigned in this namespace.

Y = 1
Y = 2
^ Lint/ConstantReassignment: Constant `Y` is already assigned in this namespace.

Z = 'hello'
Z = 'world'
^ Lint/ConstantReassignment: Constant `Z` is already assigned in this namespace.

# Reassignment via qualified path
class A
  FOO = :bar
end

A::FOO = :baz
^^^^^^^^^^^^^ Lint/ConstantReassignment: Constant `FOO` is already assigned in this namespace.

# Reassignment via absolute path
class B
  QUX = :one
end

::B::QUX = :two
^^^^^^^^^^^^^^^^ Lint/ConstantReassignment: Constant `QUX` is already assigned in this namespace.

# Reassignment via self::
self::MARKER = :first
self::MARKER = :second
^^^^^^^^^^^^^^^^ Lint/ConstantReassignment: Constant `MARKER` is already assigned in this namespace.

# Reassignment in a class using self::
class C
  self::ITEM = :a
  self::ITEM = :b
  ^^^^^^^^^^^^^^^^ Lint/ConstantReassignment: Constant `ITEM` is already assigned in this namespace.
end

# Reassignment in reopened class
module M
  class D
    VAL = :x
  end

  class D
    VAL = :y
    ^^^^^^^^^^ Lint/ConstantReassignment: Constant `VAL` is already assigned in this namespace.
  end
end

# Reassignment within another constant
class E
  ALL = [
    PART = :a,
    OTHER = :b,
    PART = :c,
    ^^^^^^^^^^ Lint/ConstantReassignment: Constant `PART` is already assigned in this namespace.
  ]
end

# Top-level constant assigned with :: from inside a class
class F
  ::GLOBAL = :first
end

GLOBAL = :second
^^^^^^^^^^ Lint/ConstantReassignment: Constant `GLOBAL` is already assigned in this namespace.

# Reassignment via nested relative namespace
module G
  class H
    DEEP = :bar
  end
end

G::H::DEEP = :baz
^^^^^^^^^^^^^^^^ Lint/ConstantReassignment: Constant `DEEP` is already assigned in this namespace.

# Reassignment via nested absolute namespace
module I
  class J
    NESTED = :bar
  end
end

::I::J::NESTED = :baz
^^^^^^^^^^^^^^^^^^ Lint/ConstantReassignment: Constant `NESTED` is already assigned in this namespace.

# Class constant reassigned from within the parent module
module K
  class L
    INNER = :bar
  end

  L::INNER = :baz
  ^^^^^^^^^^^^^ Lint/ConstantReassignment: Constant `INNER` is already assigned in this namespace.
end

# Reassignment within another constant with .freeze
class N
  ALL = [
    FLAG = :a,
    OTHER = :b,
    FLAG = :c,
    ^^^^^^^^^^ Lint/ConstantReassignment: Constant `FLAG` is already assigned in this namespace.
  ].freeze
end

# Reassignment inside a conditionally defined class
class P
  SETTING = :bar
  SETTING = :baz
  ^^^^^^^^^^ Lint/ConstantReassignment: Constant `SETTING` is already assigned in this namespace.
end unless defined?(P)
