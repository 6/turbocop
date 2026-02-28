x.include?('foo')
x.match?(/foo.*bar/)
x.match?(/\Afoo/)
x.match?(/foo\z/)
x.match?(/fo+o/)
# Regex with flags — can't use include?
x.match?(/pattern/i)
x.match?(/literal/im)
# Regex with characters outside the literal allowlist
str !~ /@/
str =~ /@/
x.match?(/@/)
str.match?(/@/)
# Regex with metachar classes — not literal
str =~ /\d/
str =~ /\w/
str =~ /\s/
str.match?(/\bword\b/)
# Regex with character class — not literal
str =~ /[abc]/
str =~ /prefix./
# No receiver on match
match(/foo/)
# str === /regex/ (wrong direction for ===)
str === /abc/
# /regex/ !~ str (regex as receiver of !~, RuboCop only flags str !~ /regex/)
/pattern/ !~ str
/one/ !~ out
/queued_/ !~ description
/\./ !~ receiver
# match/match? with multiple arguments (start position) — not flagged
/foo/.match('foo bar', 4)
/foo/.match('foo bar', -4)
'foo bar'.match(/foo/, 4)
'foo bar'.match(/foo/, -4)
/a/.match?("a", 0)
"abc".match?(/a/, 1)
