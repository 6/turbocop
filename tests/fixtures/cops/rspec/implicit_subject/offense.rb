it 'checks the subject' do
  is_expected.to be_good
  ^^^^^^^^^^^ RSpec/ImplicitSubject: Don't use implicit subject.
end
it 'checks negation' do
  should be_good
  ^^^^^^ RSpec/ImplicitSubject: Don't use implicit subject.
end
it 'checks should_not' do
  should_not be_bad
  ^^^^^^^^^^ RSpec/ImplicitSubject: Don't use implicit subject.
end
