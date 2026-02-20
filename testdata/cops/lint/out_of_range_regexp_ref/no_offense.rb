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
