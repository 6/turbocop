# nitrocop-filename: Gemfile
source 'https://rubygems.org'

gem 'allowed'
gem ENV.fetch('GEM_NAME', 'example')
gem 'frozen'.freeze
