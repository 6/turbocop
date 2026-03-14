if x > 0
  y = 1
end
while running
  process
end
while true
  break if done
end
until false
  break if done
end
case value
when 1 then "one"
when 2 then "two"
end

# Literal on rhs of && is fine
if x && 1
  top
end

# Literal in method call argument is fine
if test(42)
  top
end

# Non-toplevel and/or is fine
if (a || 1).something
  top
end

# case with non-literal when condition
case
when x > 0 then top
end

# case with expression predicate (non-literal)
case x
when 1 then "one"
end

# Literal in non-toplevel and/or as case condition
case a || 1
when b
  top
end

# begin..end while true (infinite loop idiom)
begin
  break if condition
end while true

# begin..end until false (infinite loop idiom)
begin
  break if condition
end until false

# Regex in if condition → MatchLastLineNode, not RegularExpressionNode
if /pattern/
  top
end

# Interpolated regex in if condition → InterpolatedMatchLastLineNode
if /pattern #{x}/
  top
end

# Range in if condition → FlipFlopNode, not RangeNode
# (flip-flop semantics, not literal range)
# NOTE: Cannot test this because Ruby parser warnings may interfere

# Regex in unless/while/until → also MatchLastLineNode
unless /ready/
  retry
end
