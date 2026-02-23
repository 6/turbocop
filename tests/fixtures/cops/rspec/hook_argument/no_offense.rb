before { true }
after { true }
around { |example| example.run }
before(:suite) { true }
after(:context) { true }
before(:all) { setup_database }
