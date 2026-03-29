unless a && b || c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless x || y && z
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless foo && bar || baz
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# Mixed precedence: && with and
unless a && b and c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# Mixed precedence: || with or
unless a || b or c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# Parenthesized mixed operators — RuboCop still flags these
unless (a || b) && c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless (a && b) || c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless (a || b) && (c || d)
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# AND with parenthesized OR child
unless a && (b || c)
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# Modifier form with parenthesized OR
return 0 unless width && (default_width || max_width)
         ^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.

# Chained OR with nested AND in parens
unless a || b || (c && d)
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# OR with parenthesized AND child
return false unless a || (b && c)
             ^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.

# Assignment wrapper around nested OR
return false unless (ban_reason = banned_uid? || banned_ip?) && !whitelisted_uid?
             ^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.

# Unary `!` wrapper around parenthesized OR
return false unless ready && !(foo || bar)
             ^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.

# Call argument contains nested OR inside an AND condition
return unless (block = extension.process_method[parent, block_reader || reader, attrs]) && block != parent
       ^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.

# Block body contains nested AND inside an OR condition
return false unless enabled || items.any? do |item|
             ^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  item.ready? && item.valid?
end
