ENV['X']
^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
x = ENV['X']
    ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
some_method(ENV['X'])
            ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
# Assignment in if condition: ENV['KEY'] should still be flagged
if (repo = ENV['KEY'])
           ^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('KEY', nil)` instead of `ENV['KEY']`.
  source(repo)
end
# ENV['X'] in && chain in condition: should be flagged (not a bare flag)
if ENV['A'] && ENV['B'] && other
   ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('A', nil)` instead of `ENV['A']`.
               ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('B', nil)` instead of `ENV['B']`.
  do_something
end
# case/when: both should be flagged
case ENV['X']
     ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
when ENV['Y']
     ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('Y', nil)` instead of `ENV['Y']`.
  do_something
end
# y ||= ENV['X'] should be flagged (ENV is the value, not the receiver)
y ||= ENV['X']
      ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
# y &&= ENV['X'] should be flagged
y &&= ENV['X']
      ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
# y || ENV['X'] should be flagged (ENV is RHS of ||)
y || ENV['X']
     ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
# Different key in body should be flagged even when condition guards another key
if ENV['X']
  puts ENV['Y']
       ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('Y', nil)` instead of `ENV['Y']`.
end
# ENV in condition body where condition is non-ENV
if a == b
  ENV['X']
  ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
end
# Interpolation
"#{ENV['X']}"
   ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
# ENV in body of &&-chain predicate condition should be flagged
if ENV['A'].present? && ENV['B'].present?
  config = ENV['A']
           ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('A', nil)` instead of `ENV['A']`.
end
# ENV in && condition chain (3+ elements): deeply nested ones flagged, direct child not
if ENV['A'] && ENV['B'] && ENV['C']
   ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('A', nil)` instead of `ENV['A']`.
               ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('B', nil)` instead of `ENV['B']`.
# =~ match operator is not a comparison method; ENV should be flagged
if ENV['VERSION'] =~ /-/
   ^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('VERSION', nil)` instead of `ENV['VERSION']`.
  puts "prerelease"
end
# Nested if: inner condition ENV should be flagged even when outer condition has same key
if ENV['VERSION']
  if ENV['VERSION'] =~ /-/
     ^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('VERSION', nil)` instead of `ENV['VERSION']`.
    puts "prerelease"
  end
end
# `not ENV['X']` is NOT prefix_bang — RuboCop flags it (unlike `!ENV['X']`)
not ENV['X']
    ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
if not ENV['X']
       ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
  do_something
end
# Body ENV suppressed only by nearest if ancestor, not all ancestors
# RuboCop flags ENV['X'] here because the nearest if has no ENV condition
if ENV['X']
  if other_condition
    ENV['X']
    ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
  end
end
