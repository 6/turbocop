expect(array).to be_empty
expect(array).to contain_exactly(1, 2, 3)
expect(array).to match_array([1, 2])
expect(array).to be_empty, "with a message"
expect(result).not_to be_empty
