def foo
# misaligned comment
^ Layout/CommentIndentation: Incorrect indentation detected (column 0 instead of column 2).
  x = 1
    # over-indented comment
    ^ Layout/CommentIndentation: Incorrect indentation detected (column 4 instead of column 2).
  y = 2
      # way over-indented
      ^ Layout/CommentIndentation: Incorrect indentation detected (column 6 instead of column 2).
  z = 3
end
