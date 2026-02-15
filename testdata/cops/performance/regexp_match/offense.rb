if x =~ /pattern/
   ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  do_something
end
while str =~ /\d+/
      ^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  process
end
do_something unless x =~ /re/
                    ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
