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

git 'https://example.com/my-gems.git' do
  group :development, :test do
    gem 'my_private_gem'
  end
end
