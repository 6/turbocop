describe MyClass do
  context 'when foo' do
    context 'when bar' do
      context 'when baz' do
      ^^^^^^^^^^^^^^^^^^ RSpec/NestedGroups: Maximum example group nesting exceeded [4/3].
        it { expect(true).to be(true) }
      end
    end
  end
end

describe AnotherClass do
  context 'first level' do
    context 'second level' do
      context 'exceeds max' do
      ^^^^^^^^^^^^^^^^^^^^ RSpec/NestedGroups: Maximum example group nesting exceeded [4/3].
        it { expect(1).to eq(1) }
      end
    end
  end
end

shared_examples_for 'nested behavior' do
  context 'level 1' do
    context 'level 2' do
      context 'level 3' do
        context 'level 4' do
        ^^^^^^^^^^^^^^^^^^ RSpec/NestedGroups: Maximum example group nesting exceeded [4/3].
          it { expect(subject).to be_valid }
        end
      end
    end
  end
end

describe Wrapper do
  context 'when foo' do
    context 'when bar' do
      [].each do |i|
        context 'when baz' do
        ^^^^^^^^^^^^^^^^^^ RSpec/NestedGroups: Maximum example group nesting exceeded [4/3].
        end
      end
    end
  end
end

RSpec.shared_examples 'deeply nested' do
  describe 'feature' do
    context 'level one' do
      context 'level two' do
        context 'level three' do
        ^^^^^^^^^^^^^^^^^^^^^^ RSpec/NestedGroups: Maximum example group nesting exceeded [4/3].
          it { expect(subject).to be_valid }
        end
      end
    end
  end
end

RSpec.feature 'checkout', type: :feature do
  describe 'payment' do
    context 'with credit card' do
      context 'when declined' do
      ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/NestedGroups: Maximum example group nesting exceeded [4/3].
        it { expect(page).to have_content('Error') }
      end
    end
  end
end
