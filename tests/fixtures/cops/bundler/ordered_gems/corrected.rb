gem 'rspec'
gem 'rubocop'

gem 'alpha'
gem 'zoo'

gem 'nokogiri'
gem 'puma'

gem 'rspec'
gem 'rubocop',
    '0.1.1'

platforms :jruby do
  gem "activerecord-jdbcmysql-adapter",
    git: "https://github.com/jruby/activerecord-jdbc-adapter",
    glob: "activerecord-jdbcmysql-adapter.gemspec"
  gem "activerecord-jdbcpostgresql-adapter",
    git: "https://github.com/jruby/activerecord-jdbc-adapter",
    glob: "activerecord-jdbcpostgresql-adapter.gemspec"
  gem "activerecord-jdbcsqlite3-adapter",
    git: "https://github.com/jruby/activerecord-jdbc-adapter",
    glob: "activerecord-jdbcsqlite3-adapter.gemspec"
end

group :development do
  gem 'jruby-jars', '~> 9.4.0'
  if !ENV['RACK_SRC']; gem 'jruby-rack' else gem 'jruby-rack', path: '../../target' end
  if !ENV['WARBLER_SRC']; gem 'warbler' else gem 'warbler', path: '../../warbler' end
end
