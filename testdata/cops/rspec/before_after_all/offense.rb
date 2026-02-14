before(:all) { do_something }
^^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `before(:all)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `before(:all)` are not automatically rolled back.
before(:context) { do_something }
^^^^^^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `before(:context)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `before(:context)` are not automatically rolled back.
after(:all) { do_something }
^^^^^^^^^^^ RSpec/BeforeAfterAll: Beware of using `after(:all)` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `after(:all)` are not automatically rolled back.
