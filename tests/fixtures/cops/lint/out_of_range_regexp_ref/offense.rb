puts $3
     ^^ Lint/OutOfRangeRegexpRef: $3 is out of range (no regexp capture groups detected).
/(foo)(bar)/ =~ "foobar"
puts $3
     ^^ Lint/OutOfRangeRegexpRef: $3 is out of range (2 regexp capture groups detected).
/(?<foo>FOO)(?<bar>BAR)/ =~ "FOOBAR"
puts $3
     ^^ Lint/OutOfRangeRegexpRef: $3 is out of range (2 regexp capture groups detected).
