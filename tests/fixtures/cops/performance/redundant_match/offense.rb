x.match(/pattern/)
^^^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
x.match('string')
^^^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
str.match(/\d+/)
^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
result.match(/#{expected}/)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
if str.match(/#{pattern}/)
   ^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# match in if body where the if value is not used (not last statement)
def parse(p)
  if p.look(/</)
    p.match(/</)
    ^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
    val = p.match(/[^>\n]*/)
    p.match(/>/)
    ^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
  end
  return val
end
# match in nested if body where outer if value is not used
def process(p)
  if p.look(/#/)
    p.match(/#/)
    ^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
    if p.look(/static-/)
      tag = true
      p.match(/static-/)
      ^^^^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
    end
    tag
  end
  tag
end
