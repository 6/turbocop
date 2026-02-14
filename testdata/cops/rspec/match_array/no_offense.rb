it { is_expected.to contain_exactly(content1, content2) }
it { is_expected.to match_array([content] + array) }
it { is_expected.to match_array(some_array) }
it { is_expected.to match_array([]) }
it { is_expected.to match_array }
