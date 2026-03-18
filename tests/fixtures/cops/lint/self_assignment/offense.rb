x = x
^^^^^ Lint/SelfAssignment: Self-assignment detected.

@foo = @foo
^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

$bar = $bar
^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

@@cls = @@cls
^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

FOO = FOO
^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

Mod::FOO = Mod::FOO
^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

# Compound or-assignment
foo ||= foo
^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

@bar ||= @bar
^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

@@cls ||= @@cls
^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

$glo ||= $glo
^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

# Compound and-assignment
foo &&= foo
^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

@bar &&= @bar
^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

@@cls &&= @@cls
^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

$glo &&= $glo
^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

# Multi-write self-assignment
foo, bar = foo, bar
^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo, bar = [foo, bar]
^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

# Attribute self-assignment
foo.bar = foo.bar
^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo&.bar = foo&.bar
^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

# Index self-assignment
foo["bar"] = foo["bar"]
^^^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo[:bar] = foo[:bar]
^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo[1] = foo[1]
^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo[1.2] = foo[1.2]
^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo[FOO] = foo[FOO]
^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

var = 1
foo[var] = foo[var]
^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo[@var] = foo[@var]
^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo[@@var] = foo[@@var]
^^^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

foo[$var] = foo[$var]
^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

singleton[] = singleton[]
^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.

matrix[1, 2] = matrix[1, 2]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SelfAssignment: Self-assignment detected.
