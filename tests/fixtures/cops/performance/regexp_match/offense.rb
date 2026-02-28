# =~ in if condition
if x =~ /pattern/
   ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  do_something
end
# =~ in unless condition
do_something unless x =~ /re/
                    ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
# !~ in if condition
if x !~ /pattern/
   ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `!~` when `MatchData` is not used.
  do_something
end
# !~ in unless condition (modifier)
do_something unless x !~ /re/
                    ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `!~` when `MatchData` is not used.
# .match() with regexp arg in if condition
if foo.match(/re/)
   ^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with string receiver in if condition
if "foo".match(re)
   ^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with regexp receiver in if condition
if /re/.match(foo)
   ^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with symbol receiver in if condition
if :foo.match(re)
   ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with position arg in if condition
if "foo".match(re, 1)
   ^^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# === with regexp in if condition
if /re/ === foo
   ^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `===` when `MatchData` is not used.
  do_something
end
# === with regexp with flags in if condition
if /re/i === foo
   ^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `===` when `MatchData` is not used.
  do_something
end
# =~ in elsif condition
if cond
  do_something
elsif x =~ /pattern/
      ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  do_something2
end
# =~ in ternary
x =~ /pattern/ ? do_something : do_something2
^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
# case/when with regexp
case
when /re/ === str
     ^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `===` when `MatchData` is not used.
  do_something
end
# =~ in modifier if
do_something if x =~ /re/
                ^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
# MatchData used in different method - still flag
def foo
  if x =~ /re/
     ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something
  end
end
def bar
  do_something($1)
end
# MatchData reference before the match - still flag
def baz
  do_something($1)
  if x =~ /re/
     ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something2
  end
end
