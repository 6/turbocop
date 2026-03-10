x = 'this text is too ' \
    'long'

y = 'this text contains a lot of               ' \
    'spaces'

z = "another example " \
    "without leading space"

a = "single line string"
b = 'no continuation'

# Backslash inside heredoc should not be flagged
x = <<~SQL
  SELECT * FROM users \
  WHERE id = 1
SQL

y = <<~SHELL
  echo hello \
  world
SHELL

message = %Q{expected "#{resource}[#{identity}]"} \
  " with action :#{action} to be present." \
  " Other resources:" \
  "\n\n" \
  "  " + similar_resources.join("\n  ") + "\n "

warning = "In #{resource_name} you exposed a `has_one` relationship "\
  " using the `belongs_to` class method. We think `has_one`" \
  " is more appropriate."
