it { is_expected.to match_array(array1 + array2) }
it { is_expected.to contain_exactly(content, *array) }
it { is_expected.to contain_exactly(*array, content) }
it { is_expected.to contain_exactly }
it { is_expected.to contain_exactly(1, 2, 3) }
