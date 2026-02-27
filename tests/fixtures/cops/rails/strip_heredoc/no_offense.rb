text.strip
text.chomp
text.lstrip
message.squish
text.rstrip
# strip_heredoc on regular strings is not an offense
"".strip_heredoc
"foo\nbar".strip_heredoc
"    x".strip_heredoc
text.strip_heredoc
message.strip_heredoc
query.to_s.strip_heredoc
# Heredoc chained through another method before strip_heredoc
<<-EOS.do_something.strip_heredoc
  some text
EOS
# Squiggly heredoc (no strip_heredoc needed)
<<~EOS
  some text
EOS
