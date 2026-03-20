before(:each) { do_something }
before(:example) { do_something }
after(:each) { do_something }
after(:example) { do_something }
before { do_something }
after { do_something }
config.before(:each) { do_something }
config.after(:example) { do_something }

# Method calls on objects with :all — not RSpec hooks (no block)
@state.before(:all, &@proc)
proxy.before(:all, &callback)
proxy.after(:all, &callback)
parent.after(:all, &@c)
expect(@state.before(:all)).to eq([@proc])
obj.before(:all)
obj.after(:context)
