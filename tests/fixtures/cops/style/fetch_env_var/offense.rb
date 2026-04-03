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

@name ||= if gae_instance = ENV["GAE_INSTANCE"] || ENV["CLOUD_RUN_EXECUTION"]
                                                   ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch("CLOUD_RUN_EXECUTION", nil)` instead of `ENV["CLOUD_RUN_EXECUTION"]`.

if prefix = ENV["RAILS_CACHE_ID"] || ENV["RAILS_APP_VERSION"]
                                     ^^^^^^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch("RAILS_APP_VERSION", nil)` instead of `ENV["RAILS_APP_VERSION"]`.

@current = if editor_name = ENV["RAILS_EDITOR"] || ENV["EDITOR"]
                                                   ^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch("EDITOR", nil)` instead of `ENV["EDITOR"]`.

if hosts = ENV['TEST_ES_SERVER'] || ENV['ELASTICSEARCH_HOSTS']
                                    ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('ELASTICSEARCH_HOSTS', nil)` instead of `ENV['ELASTICSEARCH_HOSTS']`.

unless token = context[:access_token] || ENV['GITHUB_TOKEN']
                                         ^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('GITHUB_TOKEN', nil)` instead of `ENV['GITHUB_TOKEN']`.

if prefix = ENV["RAILS_CACHE_ID"] || ENV["RAILS_APP_VERSION"]
                                     ^^^^^^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch("RAILS_APP_VERSION", nil)` instead of `ENV["RAILS_APP_VERSION"]`.

@current = if editor_name = ENV["RAILS_EDITOR"] || ENV["EDITOR"]
                                                   ^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch("EDITOR", nil)` instead of `ENV["EDITOR"]`.

if alternative_ids = ENV['ALT'] && alternative_ids != lang
                     ^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('ALT', nil)` instead of `ENV['ALT']`.

rng = (ENV['CI_TEST_SEED'] && ENV['CI_TEST_SEED'] != '') ? Random.new(ENV['CI_TEST_SEED'].to_i) : Random.new
       ^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('CI_TEST_SEED', nil)` instead of `ENV['CI_TEST_SEED']`.

puts unless (ENV['UNREACHABLE_ACTION_METHODS_ONLY'] || ENV['UNUSED_ROUTES_ONLY'])
                                                       ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('UNUSED_ROUTES_ONLY', nil)` instead of `ENV['UNUSED_ROUTES_ONLY']`.

($stdout.tty? || ENV['THOR_SHELL']) ? super : string
                 ^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('THOR_SHELL', nil)` instead of `ENV['THOR_SHELL']`.

$external_encoding = (ENV['LWFS_EXTERNAL_ENCODING'].nil?) ? Encoding.default_external : ENV['LWFS_EXTERNAL_ENCODING']
                                                                                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('LWFS_EXTERNAL_ENCODING', nil)` instead of `ENV['LWFS_EXTERNAL_ENCODING']`.

$log = Logger.new((ENV['LWFS_LOG_FILE'].nil?) ? STDOUT : ENV['LWFS_LOG_FILE'], 10)
                                                         ^^^^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('LWFS_LOG_FILE', nil)` instead of `ENV['LWFS_LOG_FILE']`.

log_level = ( ENV['DEBUG'] || ENV['VERBOSE'] ) ? Haconiwa::Logger::DEBUG : Haconiwa::Logger::INFO
                              ^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('VERBOSE', nil)` instead of `ENV['VERBOSE']`.

host: (ENV["MYSQL_HOST"] == "localhost") ? "127.0.0.1" : ENV["MYSQL_HOST"],
                                                         ^^^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch("MYSQL_HOST", nil)` instead of `ENV["MYSQL_HOST"]`.

SUDO = (WIN32 || ENV['SUDOLESS']) ? '': 'sudo '
                 ^^^^^^^^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('SUDOLESS', nil)` instead of `ENV['SUDOLESS']`.
