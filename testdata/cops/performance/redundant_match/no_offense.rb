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
# Result assigned to a variable (MatchData IS used)
match = if style == :spaces
          line.match(/\A\s*\t+/)
        else
          line.match(/\A\s* +/)
        end
# Result used as return value of filter_map block
entries.filter_map { |e| e.match(%r{pattern}) }
# Result used in assignment inside an if body
match = line[pos..]&.match(/\S+/)
# Safe navigation (&.) is not flagged (RuboCop's RESTRICT_ON_SEND only matches regular calls)
line&.match(/pattern/)
# ||= assignment: result IS used
in_ruby_section ||= line.match(/pattern/)
