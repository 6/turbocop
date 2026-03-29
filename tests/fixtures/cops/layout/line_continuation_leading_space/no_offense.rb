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

result = "prefix " \
  "continued" + extra_info

# %Q{} percent string head + continuation with leading spaces:
# RuboCop's autocorrect crashes (LINE_1_ENDING doesn't match `} \`)
# so the entire dstr processing is aborted, reporting 0 offenses.
message = %Q{expected "#{resource}[#{identity}]"} \
  " with action :#{action} to be present." \
  " Other #{resource} resources:" \
  "\n\n" \
  "  " + similar_resources.join("\n  ") + "\n "
