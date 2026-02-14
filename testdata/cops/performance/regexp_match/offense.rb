x =~ /pattern/
^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
x =~ y
^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
str =~ /\d+/
^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
