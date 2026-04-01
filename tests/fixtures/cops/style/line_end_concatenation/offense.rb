top = "test" +
             ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"top"
msg = "hello" <<
              ^^ Style/LineEndConcatenation: Use `\` instead of `<<` to concatenate multiline strings.
"world"
x = "foo" +
          ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"bar"

'These issues has been marked as fixed either manually or by '+
                                                              ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
'not being found by future scan revisions.'

status = [
  'alert-error',
  'The server takes too long to respond to the scan requests,' +
                                                               ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
    ' this will severely diminish performance.']

x = 'HTTP request concurrency has been drastically throttled down ' +
                                                                    ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
    "(from the maximum of #{max}) due to very high server" +
                                                           ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
    " response times, this will severely decrease performance."

where( 'requires_verification = ? AND verified = ? AND ' +
                                                         ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
           'false_positive = ? AND fixed = ?', true, true, false, false )

where( 'requires_verification = ? AND verified = ? AND '+
                                                        ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
           ' false_positive = ? AND fixed = ?', true, false, false, false )

statuses = {
  form_not_visible: 'The form was located but its DOM element is not ' <<
                                                                       ^^ Style/LineEndConcatenation: Use `\` instead of `<<` to concatenate multiline strings.
      'visible and thus cannot be submitted.',
}

config = {
  description: 'Forces the proxy to only extract vector '+
                                                         ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
    'information from observed HTTP requests and not analyze responses.',
}
