top = "test" +
             ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"top"
msg = "hello" <<
              ^^ Style/LineEndConcatenation: Use `\` instead of `<<` to concatenate multiline strings.
"world"
x = "foo" +
          ^ Style/LineEndConcatenation: Use `\` instead of `+` to concatenate multiline strings.
"bar"
