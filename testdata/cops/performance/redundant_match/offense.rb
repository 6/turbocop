x.match(/pattern/)
^^^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
x.match('string')
^^^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
str.match(/\d+/)
^^^^^^^^^^^^^^^^ Performance/RedundantMatch: Use `match?` instead of `match` when `MatchData` is not used.
