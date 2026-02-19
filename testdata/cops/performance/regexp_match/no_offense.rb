x.match?(/pattern/)
x.match?(y)
x == y
x.include?("pattern")
x.start_with?("p")
# =~ not in a condition context — not flagged
result = x =~ /pattern/
x =~ /re/
arr.select { |s| s =~ /re/ }
def matches?(val)
  val =~ /re/
end
# =~ in condition but MatchData is used — not flagged
if x =~ /pattern/
  Regexp.last_match(1).downcase
end
if str =~ /(\d+)/
  puts $1
end
# =~ or !~ inside && chain — not flagged (RuboCop only checks top-level condition)
if foo && bar =~ /re/
  do_something
end
if request.path !~ /pattern/ && other
  redirect
end
