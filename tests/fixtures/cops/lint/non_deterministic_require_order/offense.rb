Dir["./lib/**/*.rb"].each do |file|
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonDeterministicRequireOrder: Sort files before requiring them.
  require file
end
Dir.glob(Rails.root.join('test', '*.rb')).each do |file|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonDeterministicRequireOrder: Sort files before requiring them.
  require file
end
Dir["./lib/**/*.rb"].each(&method(:require))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonDeterministicRequireOrder: Sort files before requiring them.
