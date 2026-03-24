'foo'.gsub('bar', 'baz')
'foo'.match(/ba./)
'foo'.split(/,+/)
'foo'.gsub(/\d/, '')
x = 'hello'
y = x.match?('world')

# match and match? are not flagged (not in target methods)
'foo'.match(/bar/)
'foo'.match?(/bar/)

# end_with? not flagged
'foo'.end_with?(/bar/)

# Single space regexp is idiomatic
'foo'.split(/ /)

# %r{} syntax should not be flagged (intentional style choice)
string.gsub(%r{"}) { '\\"' }
match_rule_quoted = Regexp.quote(match_rule).gsub(%r{\\\*}, '.*')
'foo'.sub(%r{bar}, 'baz')
'foo'.split(%r{,})
'foo'.gsub(%r{\.}, '-')

# @ is not in RuboCop's LITERAL_REGEX whitelist
var.to_s.gsub(/@/, '')
'foo'.scan(/@/)

# Non-ASCII chars are not in whitelist
str.gsub(/ß/, 'ss')
str.gsub(/⌘/, 'cmd')

# $ is not in whitelist
url.scan(/\$@\$/)
