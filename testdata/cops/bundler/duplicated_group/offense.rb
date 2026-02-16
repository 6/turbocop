source 'https://rubygems.org'

group :development do
  gem 'rubocop'
end

group :development do
^ Bundler/DuplicatedGroup: Gem group `:development` already defined on line 3 of the Gemfile.
  gem 'rubocop-rails'
end

group :test do
  gem 'rspec'
end

group :test do
^ Bundler/DuplicatedGroup: Gem group `:test` already defined on line 11 of the Gemfile.
  gem 'factory_bot'
end

group :development do
^ Bundler/DuplicatedGroup: Gem group `:development` already defined on line 3 of the Gemfile.
  gem 'pry'
end
