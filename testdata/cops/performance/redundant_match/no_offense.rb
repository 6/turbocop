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
