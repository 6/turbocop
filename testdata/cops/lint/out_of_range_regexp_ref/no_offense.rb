/(foo)(bar)/ =~ "foobar"
puts $1
puts $2
/(?<foo>FOO)(?<bar>BAR)/ =~ "FOOBAR"
puts $1
puts $2
"foobar"[/(foo)(bar)/]
puts $2
