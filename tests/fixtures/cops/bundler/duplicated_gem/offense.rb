source 'https://rubygems.org'
gem 'rubocop'
gem 'rubocop'
^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `rubocop` requirements already given on line 2 of the Gemfile.

gem 'rails'
gem 'puma'
gem 'rails'
^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `rails` requirements already given on line 5 of the Gemfile.

gem 'nokogiri', '~> 1.0'
gem 'nokogiri'
^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `nokogiri` requirements already given on line 9 of the Gemfile.

# Gems inside git blocks within case/when should be flagged as duplicates
# because git blocks are not direct children of the conditional branch
case rails
when /\//
  gem 'activesupport', path: "#{rails}/activesupport"
  gem 'activemodel', path: "#{rails}/activemodel"
  gem 'activerecord', path: "#{rails}/activerecord", require: false
when /^v/
  git 'https://github.com/rails/rails.git', tag: rails do
    gem 'activesupport'
    ^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `activesupport` requirements already given on line 16 of the Gemfile.
    gem 'activemodel'
    ^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `activemodel` requirements already given on line 17 of the Gemfile.
    gem 'activerecord', require: false
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `activerecord` requirements already given on line 18 of the Gemfile.
  end
else
  git 'https://github.com/rails/rails.git', branch: rails do
    gem 'activesupport'
    ^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `activesupport` requirements already given on line 16 of the Gemfile.
    gem 'activemodel'
    ^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `activemodel` requirements already given on line 17 of the Gemfile.
    gem 'activerecord', require: false
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `activerecord` requirements already given on line 18 of the Gemfile.
  end
end

# Gem in group block — group does not provide conditional exemption
gem 'webpacker'
group :development do
  gem 'webpacker', path: '/path/to/gem'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `webpacker` requirements already given on line 34 of the Gemfile.
end

# Identical gem in group block + conditional — first decl is NOT conditional,
# so structural equality does not apply (RuboCop's conditional_declaration?
# checks nodes[0]'s ancestor first).
group :development do
  gem 'pg', '1.5.1'
end
if ENV['DEPLOY_METHOD'] == "docker"
  gem 'pg', '1.5.1'
  ^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `pg` requirements already given on line 43 of the Gemfile.
end

# Identical gems inside blocks within case/when — blocks break the
# conditional direct-child relationship so these should be flagged.
# The first gem's non-begin ancestor is :block (not :when), so
# conditional_declaration? returns false.
case rails
when /\//
  path rails do
    gem 'oj'
  end
when /^v/
  git 'https://github.com/rails/rails.git', tag: rails do
    gem 'oj'
    ^^^^^^^^ Bundler/DuplicatedGem: Gem `oj` requirements already given on line 56 of the Gemfile.
  end
else
  git 'https://github.com/rails/rails.git', branch: rails do
    gem 'oj'
    ^^^^^^^^ Bundler/DuplicatedGem: Gem `oj` requirements already given on line 56 of the Gemfile.
  end
end

# Modifier if makes the gem's non-begin ancestor be the modifier if node,
# not the enclosing block-form if. Different source means no structural
# equality match, so the duplicate is flagged.
if !defined?(JRUBY_VERSION)
  gem 'google-protobuf', '< 3.12' if RUBY_VERSION < '2.5'
  gem 'google-protobuf', '< 3.23' if RUBY_VERSION < '2.7'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `google-protobuf` requirements already given on line 72 of the Gemfile.
end

# Gem inside source block in if, with different gem in else — first gem's
# ancestor is :block, so not conditional.
if ENV.has_key?('OS_BUILD_ID')
  source 'https://rubygems.vpsfree.cz' do
    gem 'libosctl'
  end
else
  gem 'libosctl', path: '../libosctl'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `libosctl` requirements already given on line 80 of the Gemfile.
end

# Gem in git block inside if, direct gem in else
if version == 'master'
  git 'https://github.com/rails/rails.git' do
    gem 'myrails'
  end
else
  gem 'myrails', "~> #{version}.0"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `myrails` requirements already given on line 89 of the Gemfile.
end

# Gem with do...end block in if/else — the block makes the first gem's
# non-begin ancestor :block (not :if) in Parser gem, so conditional_declaration?
# returns false and all duplicates are flagged unconditionally.
if ENV['ROM_SOURCE']
  gem 'datamapper', github: 'rom-rb/rom', branch: 'master' do
    gem 'datamapper-core'
  end
else
  gem "datamapper"
  ^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `datamapper` requirements already given on line 99 of the Gemfile.
end

# Modifier-if gem in if/elsif/else — modifier if is the root conditional,
# other gem has different structure, not exempted
if RUBY_VERSION =~ /^1\.?9/
  gem 'json_pure', '<=2.0.1'
  gem 'code-check', '0.41.2' if RUBY_VERSION =~ /^1\.?9/
elsif RUBY_VERSION =~ /^1\.?8/
  gem 'json_pure', '< 2.0.0'
else
  gem 'code-check'
  ^^^^^^^^^^^^^^^^^ Bundler/DuplicatedGem: Gem `code-check` requirements already given on line 110 of the Gemfile.
