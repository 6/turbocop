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
