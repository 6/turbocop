before { true }
after { true }
around { |example| example.run }
before(:suite) { true }
after(:context) { true }
before(:all) { setup_database }

# Explicit block-pass (`&handler`) is not an any_block hook and should be ignored.
state.before(:each, &handler)
state.after(:example, &handler)

# Multi-arg hooks: :each/:example with additional args should NOT be flagged.
# RuboCop's NodePattern only matches when the scope symbol is the sole argument.
before(:each, :special_tag) do
  setup
end

around(:each, :allow_forgery_protection) do |example|
  example.run
end

after(:each, type: :system) do
  cleanup
end

config.before(:example, js: true) do
  setup
end

config.after(:each, :allow_forgery_protection) do |example|
  example.run
end
