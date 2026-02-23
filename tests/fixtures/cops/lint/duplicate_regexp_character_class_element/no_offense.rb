r = /[xy]/
r = /[abc]/
r = /[0-9]/
r = /foo/
r = /[a-z]/
# POSIX character classes should not trigger false positives
r = /[[:digit:][:upper:]_]+/
r = /[[:alnum:]:]/
r = /[[:alpha:][:digit:]]+/
r = /\(#([[:digit:]]+)\)/
# Unicode property escapes should not trigger false positives
r = /[^\p{White_Space}<>()?]/
r = /[\p{L}\p{N}_-]/
r = /[^\P{ASCII}]/
# Character class intersection should not trigger false positives
r = /[\S&&[^\\]]/
r = /[a-z&&[^aeiou]]/
# Hex escape sequences in character classes should not trigger false positives
r = /[\x00-\x1F\x7F]/
r = /[\x20-\x7E]/
r = /[\\<>@"!#$%&*+=?^`{|}~:;]/
# Octal escapes should not trigger false positives
r = /[\0\1\2]/
