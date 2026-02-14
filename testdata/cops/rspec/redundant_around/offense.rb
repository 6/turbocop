around do |example|
^^^^^^^^^^^^^^^^^^^ RSpec/RedundantAround: Remove redundant `around` hook.
  example.run
end

around(&:run)
^^^^^^^^^^^^^ RSpec/RedundantAround: Remove redundant `around` hook.

around do |ex|
^^^^^^^^^^^^^^ RSpec/RedundantAround: Remove redundant `around` hook.
  ex.run
end
