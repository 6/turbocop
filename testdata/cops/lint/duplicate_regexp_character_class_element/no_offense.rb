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
