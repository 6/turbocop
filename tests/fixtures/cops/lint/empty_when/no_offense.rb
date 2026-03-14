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
# Inline comment on when line (AllowComments: true by default)
case line
when /^\s+not a dynamic executable$/ # ignore non-executable files
when :other
  handle(line)
end
case char
when 'C' ; # ignore right key
when 'D' ; # ignore left key
else
  handle(char)
end
case value
when 2 then # comment
when 3
  do_something
end
# Multi-line when condition with comment body
case field_param
when "description",
     "status_explanation",
     /\Aagenda_items_\d+_notes\z/
  # no additional checks
when "other"
  validate(field_param)
end
case sym
when :controller, :requirements, :singular, :path_prefix, :as,
  :path_names, :shallow, :name_prefix, :member_path, :nested_member_path,
  :belongs_to, :conditions, :active_scaffold
  #should be able to skip
when :other
  handle(sym)
end
case line
when /\AWEBrick [\d.]+/,
     /\Aruby ([\d.]+)/,
     /\ARackup::Handler::WEBrick is mounted/,
     /\Aclose TCPSocket/,
  # ignored
when /\Aother/
  process(line)
end
# Heredoc condition with comment body (AllowComments: true)
case content
when <<~TEXT
  expected content
TEXT
  # heredoc match is intentionally ignored
when "other"
  process(content)
end
