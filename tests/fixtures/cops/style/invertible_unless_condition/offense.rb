# Simple operator conditions — block form (keyword and node start are the same)
unless x != y
^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x == y` over `unless x != y`.
  do_something
end
unless foo.even?
^^^^^^ Style/InvertibleUnlessCondition: Prefer `if foo.odd?` over `unless foo.even?`.
  bar
end

# Simple operator conditions — modifier form (node starts at body, not keyword)
do_something unless x > 0
^^^^^^^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x <= 0` over `unless x > 0`.

# Negation with !
foo unless !bar
^^^ Style/InvertibleUnlessCondition: Prefer `if bar` over `unless !bar`.
foo unless !!bar
^^^ Style/InvertibleUnlessCondition: Prefer `if !bar` over `unless !!bar`.

# Methods without explicit receiver (implicit self)
foo unless odd?
^^^ Style/InvertibleUnlessCondition: Prefer `if even?` over `unless odd?`.

# Complex compound conditions (AND/OR)
foo unless x != y && x.odd?
^^^ Style/InvertibleUnlessCondition: Prefer `if x == y || x.even?` over `unless x != y && x.odd?`.
foo unless x != y || x.even?
^^^ Style/InvertibleUnlessCondition: Prefer `if x == y && x.odd?` over `unless x != y || x.even?`.

# Parenthesized conditions
foo unless ((x != y))
^^^ Style/InvertibleUnlessCondition: Prefer `if ((x == y))` over `unless ((x != y))`.

# Other invertible operators
do_something unless x >= 10
^^^^^^^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x < 10` over `unless x >= 10`.
do_something unless x <= 5
^^^^^^^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x > 5` over `unless x <= 5`.
do_something unless x < 3
^^^^^^^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x >= 3` over `unless x < 3`.
do_something unless x !~ /pattern/
^^^^^^^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x =~ /pattern/` over `unless x !~ /pattern/`.

# Predicate methods
foo unless items.zero?
^^^ Style/InvertibleUnlessCondition: Prefer `if items.nonzero?` over `unless items.zero?`.
foo unless items.any?
^^^ Style/InvertibleUnlessCondition: Prefer `if items.none?` over `unless items.any?`.
foo unless items.none?
^^^ Style/InvertibleUnlessCondition: Prefer `if items.any?` over `unless items.none?`.
foo unless items.nonzero?
^^^ Style/InvertibleUnlessCondition: Prefer `if items.zero?` over `unless items.nonzero?`.

# Complex nested compound condition
foo unless x != y && (((x.odd?) || (((y >= 5)))) || z.zero?)
^^^ Style/InvertibleUnlessCondition: Prefer `if x == y || (((x.even?) && (((y < 5)))) && z.nonzero?)` over `unless x != y && (((x.odd?) || (((y >= 5)))) || z.zero?)`.

# All-uppercase constant with < is NOT inheritance (so it IS invertible)
foo unless x < FOO
^^^ Style/InvertibleUnlessCondition: Prefer `if x >= FOO` over `unless x < FOO`.

# Multi-line modifier unless — offense reported at start of expression (line 1), not at keyword
errors.add(
^^^^^^^^^^ Style/InvertibleUnlessCondition: Prefer `if x == y` over `unless x != y`.
  :base,
  "subject and expected_receive_period_in_days are required"
) unless x != y

return nil unless time_of_day.present? && time_of_day.values.any?(&:positive?)
^ Style/InvertibleUnlessCondition: Prefer `if time_of_day.blank? || time_of_day.values.none?(&:positive?)` over `unless time_of_day.present? && time_of_day.values.any?(&:positive?)`.

return nil unless day_of_week.present? && day_of_week.any?(&:positive?)
^ Style/InvertibleUnlessCondition: Prefer `if day_of_week.blank? || day_of_week.none?(&:positive?)` over `unless day_of_week.present? && day_of_week.any?(&:positive?)`.

return nil unless seasonality.present? && seasonality.values.any?(&:positive?)
^ Style/InvertibleUnlessCondition: Prefer `if seasonality.blank? || seasonality.values.none?(&:positive?)` over `unless seasonality.present? && seasonality.values.any?(&:positive?)`.

return nil unless day_of_week.present? && day_of_week.any?(&:positive?)
^ Style/InvertibleUnlessCondition: Prefer `if day_of_week.blank? || day_of_week.none?(&:positive?)` over `unless day_of_week.present? && day_of_week.any?(&:positive?)`.

return unless @queries.values.any?(&:current_context?) || @connections.any?(&:current_context?)
^ Style/InvertibleUnlessCondition: Prefer `if @queries.values.none?(&:current_context?) && @connections.none?(&:current_context?)` over `unless @queries.values.any?(&:current_context?) || @connections.any?(&:current_context?)`.

return unless target.dependent_targets_for_test_spec(test_spec).any?(&:uses_swift?)
^ Style/InvertibleUnlessCondition: Prefer `if target.dependent_targets_for_test_spec(test_spec).none?(&:uses_swift?)` over `unless target.dependent_targets_for_test_spec(test_spec).any?(&:uses_swift?)`.

result[f] = r unless r.any?(&:empty?)
^ Style/InvertibleUnlessCondition: Prefer `if r.none?(&:empty?)` over `unless r.any?(&:empty?)`.

errors.add(:base, :scripts_absent) unless fields.any?(&:present?)
^ Style/InvertibleUnlessCondition: Prefer `if fields.none?(&:present?)` over `unless fields.any?(&:present?)`.
