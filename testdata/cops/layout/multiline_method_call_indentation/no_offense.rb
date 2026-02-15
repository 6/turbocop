foo.bar.baz

foo
  .bar

foo
  .bar
  .baz

result = something
  .where(x: 1)
  .order(:name)
  .limit(10)

obj.method1
   .method2
   .method3

# First continuation dot when all previous dots are inline
ActiveRecord::Base.configurations.configs_for(env_name: Rails.env).first.configuration_hash
  .dup
  .tap { |config| config['pool'] = 1 }
