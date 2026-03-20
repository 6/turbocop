before(:all) { do_something }
^^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `before(:all)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `before(:all)` are not automatically rolled back.
before(:context) { do_something }
^^^^^^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `before(:context)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `before(:context)` are not automatically rolled back.
after(:all) { do_something }
^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `after(:all)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `after(:all)` are not automatically rolled back.
config.before(:all) { setup }
^^^^^^^^^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `config.before(:all)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `config.before(:all)` are not automatically rolled back.
context.after(:context) { cleanup }
^^^^^^^^^^^^^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `context.after(:context)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `context.after(:context)` are not automatically rolled back.
config.before :all do |group|
end
# nitrocop-expect: 6:0 RSpec/BeforeAfterAll: Beware of using `config.before :all` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `config.before :all` are not automatically rolled back.
