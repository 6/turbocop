1..2
'a'..'z'
:bar..:baz
a..b
-a..b
(x || 1)..2
match.begin(0)...match.end(0)
source.index('[')..source.index(']')
a.foo..b.bar
obj[0]..obj[1]
# Parenthesized operator expressions are fine
(a + 1)..b
(a * 2)..b
(MESSAGES_PER_CONVERSATION + 5)..10
[1, 0]...[1, 6]
# Rational literals (e.g., 1/3r) are acceptable boundaries
1/10r..1/3r
0/1r..1/1r
# begin...end blocks are acceptable range boundaries (RuboCop's begin_type?)
begin; compute_min; end..begin; compute_max; end
# Method calls with block arguments (&) are acceptable — Prism puts &block in
# CallNode.block(), but it's a BlockArgumentNode, not an actual block body.
records.select(&:active?)..records.reject(&:archived?)

# FP: Arithmetic operators on variables as range boundaries
def get_text(start)
  @string[start..@pos-1]
end

# FP: Range boundaries that are themselves ranges (endless/beginless)
1.. ..1
1... ...1
