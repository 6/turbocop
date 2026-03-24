# Simple operator conditions
unless x != y
^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x == y` over `unless x != y`.
  do_something
end
do_something unless x > 0
             ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x <= 0` over `unless x > 0`.
unless foo.even?
^^^^^^ Style/InvertibleUnlessCondition: Prefer `if foo.odd?` over `unless foo.even?`.
  bar
end

# Negation with !
foo unless !bar
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if bar` over `unless !bar`.
foo unless !!bar
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if !bar` over `unless !!bar`.

# Methods without explicit receiver (implicit self)
foo unless odd?
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if even?` over `unless odd?`.

# Complex compound conditions (AND/OR)
foo unless x != y && x.odd?
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x == y || x.even?` over `unless x != y && x.odd?`.
foo unless x != y || x.even?
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x == y && x.odd?` over `unless x != y || x.even?`.

# Parenthesized conditions
foo unless ((x != y))
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if ((x == y))` over `unless ((x != y))`.

# Other invertible operators
do_something unless x >= 10
             ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x < 10` over `unless x >= 10`.
do_something unless x <= 5
             ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x > 5` over `unless x <= 5`.
do_something unless x < 3
             ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x >= 3` over `unless x < 3`.
do_something unless x !~ /pattern/
             ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x =~ /pattern/` over `unless x !~ /pattern/`.

# Predicate methods
foo unless items.zero?
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if items.nonzero?` over `unless items.zero?`.
foo unless items.any?
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if items.none?` over `unless items.any?`.
foo unless items.none?
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if items.any?` over `unless items.none?`.
foo unless items.nonzero?
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if items.zero?` over `unless items.nonzero?`.

# Complex nested compound condition
foo unless x != y && (((x.odd?) || (((y >= 5)))) || z.zero?)
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x == y || (((x.even?) && (((y < 5)))) && z.nonzero?)` over `unless x != y && (((x.odd?) || (((y >= 5)))) || z.zero?)`.

# All-uppercase constant with < is NOT inheritance (so it IS invertible)
foo unless x < FOO
    ^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x >= FOO` over `unless x < FOO`.
