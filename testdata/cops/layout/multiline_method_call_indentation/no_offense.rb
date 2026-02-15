foo.bar.baz

foo
  .bar

foo
  .bar
  .baz

obj.method1
   .method2
   .method3

# In method body
def foo
  query
    .select('foo')
    .limit(1)
end

# Block-based expect chain
expect(response)
  .to have_http_status(200)
  .and have_http_link_header('http://example.com')

# First continuation dot when all previous dots are inline
ActiveRecord::Base.configurations.configs_for(env_name: Rails.env).first.configuration_hash
  .dup
  .tap { |config| config['pool'] = 1 }
