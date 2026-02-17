setup do
^^^^^ RSpecRails/AvoidSetupHook: Use `before` instead of `setup`.
  allow(foo).to receive(:bar)
end

setup do
^^^^^ RSpecRails/AvoidSetupHook: Use `before` instead of `setup`.
  create_users
end

setup do
^^^^^ RSpecRails/AvoidSetupHook: Use `before` instead of `setup`.
  prepare_data
  run_migrations
end
