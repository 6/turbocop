x.match?(/pattern/)
x.match?('string')
x =~ /pattern/
x.scan(/pattern/)
match(/pattern/)
# MatchData is used: chained, indexed, assigned, block
result = x.match(/pattern/)
x.match(/pattern/)[1]
x.match(/pattern/).to_s
x.match(/pattern/) { |m| m[1] }
str&.match(/pattern/)&.captures
# No literal on either side — not flagged (matches RuboCop behavior)
pattern.match(variable)
ignored_errors.any? { |pat| pat.match(error.message) }
expect(subject.match(input)).to be_nil
expect(subject.match('string')).to be_nil
segment.match(SOME_CONSTANT)
