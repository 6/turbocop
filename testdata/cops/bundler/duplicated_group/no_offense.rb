source 'https://rubygems.org'

group :development do
  gem 'rubocop'
end

group :test do
  gem 'rspec'
end

group :development, :test do
  gem 'factory_bot'
end
