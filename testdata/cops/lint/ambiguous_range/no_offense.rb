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
# Operator expressions are acceptable when RequireParenthesesForMethodChains is false (default)
MESSAGES_PER_CONVERSATION..MESSAGES_PER_CONVERSATION + 5
a + 1..b
a * 2..b
