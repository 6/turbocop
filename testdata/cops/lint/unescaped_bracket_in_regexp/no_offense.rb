/]/
/abc\]123/
/[abc]/
/(?<=[<>=:])/
%r{]}
%r{abc\]123}
%r{[abc]}
%r{(?<=[<>=:])}

# Nested character classes
/[a[b]]/
# POSIX classes inside character classes
/[[:blank:]]/
/[[:alpha:][:digit:]]/
# Complex nested character classes
/^@{0,2}[\d[[:lower:]]_]+$/
/^@{0,2}(?:_|_?[[[:lower:]]][\d[[:lower:]][[:upper:]]]*)[!?=]?$/
