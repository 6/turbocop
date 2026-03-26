ENV['SECRET_KEY']
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV["DATABASE_URL"]
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV.fetch('REDIS_URL')
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
::ENV.fetch('API_KEY')
^^^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV['FOO'] = 'bar'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
::ENV['QUX'] = 'val'
^^^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV.store('KEY', 'value')
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV.delete('KEY')
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV['BUNDLE_GEMFILE'] ||= File.expand_path('../Gemfile', __dir__)
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV['RAILS_ENV'] ||= 'test'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
::ENV['APP_ENV'] ||= 'development'
^^^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV['COUNTER'] &&= 'updated'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV['COUNT'] += '1'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV['A'], ENV['B'] = a, b
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
          ^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV.fetch('KEY', DEFAULT_VALUE)
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
                 ^^^^^^^^^^^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV.fetch(
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
  "PG_EXTRAS_TABLE_CACHE_HIT_MIN_EXPECTED",
  PG_EXTRAS_TABLE_CACHE_HIT_MIN_EXPECTED
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
).to_f
ENV['FOO'] = SOME_CONST
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
             ^^^^^^^^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.

argv.insert(0, *ENV['RDOCOPT'].split) if ENV['RDOCOPT']
                ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
                                         ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
argv = ENV['RI'].to_s.split.concat argv
       ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV['PATH'].split(File::PATH_SEPARATOR).any? do |path|
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
pagers = [ENV['RI_PAGER'], ENV['PAGER'], 'pager', 'less', 'more']
          ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
                           ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
homedir ||= ENV['HOME'] ||
            ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV['USERPROFILE'] || ENV['HOMEPATH'] # for 1.8 compatibility
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
                      ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
terminal_width = (ENV['COLUMNS'] || 80).to_i
                  ^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
