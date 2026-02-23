do_something(<<~'EOS')
             ^^^^^^^^ Style/RedundantHeredocDelimiterQuotes: Remove the redundant heredoc delimiter quotes, use `<<~EOS` instead.
  no string interpolation style text
EOS

do_something(<<-"EOS")
             ^^^^^^^^ Style/RedundantHeredocDelimiterQuotes: Remove the redundant heredoc delimiter quotes, use `<<-EOS` instead.
  plain text here
EOS

do_something(<<"EOS")
             ^^^^^^^ Style/RedundantHeredocDelimiterQuotes: Remove the redundant heredoc delimiter quotes, use `<<EOS` instead.
  just plain text
EOS
