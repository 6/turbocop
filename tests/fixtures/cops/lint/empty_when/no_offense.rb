case foo
when 1
  do_something
when 2
  do_other
end
case bar
when :a
  handle_a
end
# Empty when with comment — not flagged when AllowComments is true (default)
case storage
when :s3
  process_s3
when :fog, :azure
  # Not supported
when :filesystem
  process_fs
end
