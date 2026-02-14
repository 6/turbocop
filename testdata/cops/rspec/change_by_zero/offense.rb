RSpec.describe 'test' do
  it 'detects change.by(0)' do
    expect { foo }.to change(Foo, :bar).by(0)
                      ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ChangeByZero: Prefer `not_to change` over `to change.by(0)`.
  end

  it 'detects block change.by(0)' do
    expect { foo }.to change { Foo.bar }.by(0)
                      ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ChangeByZero: Prefer `not_to change` over `to change.by(0)`.
  end

  it 'detects another change.by(0)' do
    expect { bar }.to change(Bar, :baz).by(0)
                      ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ChangeByZero: Prefer `not_to change` over `to change.by(0)`.
  end
end
