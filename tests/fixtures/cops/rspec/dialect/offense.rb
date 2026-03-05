context 'display name presence' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Dialect: Prefer `describe` over `context`.
end

context 'another test' do
^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Dialect: Prefer `describe` over `context`.
end

RSpec.context 'via RSpec' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Dialect: Prefer `describe` over `context`.
end

it 'raises an error' do
  expect { subject }.to raise_exception(StandardError)
                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Dialect: Prefer `raise_error` over `raise_exception`.
end

expect { run }.to raise_exception(ArgumentError, 'bad input')
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Dialect: Prefer `raise_error` over `raise_exception`.
