/(foo)(bar)/ =~ "foobar"
puts $1
puts $2
/(?<foo>FOO)(?<bar>BAR)/ =~ "FOOBAR"
puts $1
puts $2
"foobar"[/(foo)(bar)/]
puts $2

# Regexp is a constant reference — captures can't be determined statically
PATTERN = /(\w+)/
str =~ PATTERN
puts $1
str.match(PATTERN)
puts $1

# gsub/sub with a variable regexp arg — captures can't be determined
pattern = Regexp.new('(\w+)\s+(\w+)')
str.gsub(pattern) { "#{$1}-#{$2}" }
str.sub(pattern) { $1 }

# scan with literal zero-capture regexp, then gsub with variable regexp
str.scan(/^##.*/) do |line|
  line.gsub(pattern) { $1 }
end
