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

"      " + "new_#{name} = FFaker::Lorem.paragraphs(1).join(\"\") \n" +
                                                                     ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"      find(\"[name='#{testing_name}[#{name.to_s}]']\").fill_in(with: new_#{name.to_s})"

em.map { |m| "+" + m.to_s.sub(/.*:/, "") } * "" +
                                                ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
" offset=#{interval.first}"

"FFFFFFFF" "FFFFFFFF" "C90FDAA2" "2168C234" +
                                            ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"C4C6628B" "80DC1CD1" "29024E08" "8A67CC74" +
                                            ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"020BBEA6" "3B139B22" "514A0879" "8E3404DD" +
                                            ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"EF9519B3" "CD3A431B" "302B0A6D" "F25F1437" +
                                            ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"4FE1356D" "6D51C245" "E485B576" "625E7EC6" +
                                            ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"F44C42E9" "A637ED6B" "0BFF5CB6" "F406B7ED" +
                                            ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"EE386BFB" "5A899FA5" "AE9F2411" "7C4B1FE6"

x = %Q'_TEXT_ "#{text}"' +
                         ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"end"
