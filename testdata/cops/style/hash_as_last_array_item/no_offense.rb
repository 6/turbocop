[1, 2, { one: 1, two: 2 }]
[{ one: 1 }, { two: 2 }]
[1, {}]
foo(one: 1, two: 2)
[1, 2, 3]
[1, 2].each { |x| puts x }

# Implicit arrays from method args - not flagged
method_call 1, 2, key: value

# All elements are keyword hashes - not flagged
[auto_assignment_config: [:max_assignment_limit]]
[attribute_key: 'country_code', filter_operator: 'equal_to', values: ['US']]
