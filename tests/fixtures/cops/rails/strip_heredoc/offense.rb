<<-EOS.strip_heredoc
^^^^^^^^^^^^^^^^^^^^ Rails/StripHeredoc: Use squiggly heredoc (`<<~`) instead of `strip_heredoc`.
  some text
EOS

<<EOS.strip_heredoc
^^^^^^^^^^^^^^^^^^^ Rails/StripHeredoc: Use squiggly heredoc (`<<~`) instead of `strip_heredoc`.
  some text
EOS

<<~EOS.strip_heredoc
^^^^^^^^^^^^^^^^^^^^ Rails/StripHeredoc: Use squiggly heredoc (`<<~`) instead of `strip_heredoc`.
  some text
EOS
