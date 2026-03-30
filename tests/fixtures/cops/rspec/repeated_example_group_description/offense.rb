describe 'doing x' do
^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [5]
  it { something }
end

describe 'doing x' do
^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [1]
  it { other }
end

context 'when awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [13]
  it { thing }
end

context 'when awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [9]
  it { other_thing }
end

# Repeated groups inside a module wrapper (FN case)
module MyModule
  describe 'feature' do
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [23]
    it { works }
  end

  describe 'feature' do
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [19]
    it { also_works }
  end
end

# Repeated groups inside a class wrapper
class MySpec
  context 'when valid' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [34]
    it { passes }
  end

  context 'when valid' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [30]
    it { also_passes }
  end
end

# Quote style difference (double vs single) should still flag as duplicate
describe "quoted with double" do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [44]
  it { something }
end

describe 'quoted with double' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [40]
  it { other }
end

describe 'conditional repeats' do
  if enabled
    context 'with no upstream package source' do
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [54]
      it { works }
    end

    context 'with no upstream package source' do
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [50]
      it { also_works }
    end
  end
end
