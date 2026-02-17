assert_equal(a, b)
^^^^^^^^^^^^^^^^^^ RSpecRails/MinitestAssertions: Use `expect(b).to eq(a)`.
refute_equal(a, b)
^^^^^^^^^^^^^^^^^^ RSpecRails/MinitestAssertions: Use `expect(b).not_to eq(a)`.
assert_nil a
^^^^^^^^^^^^ RSpecRails/MinitestAssertions: Use `expect(a).to eq(nil)`.
refute_empty(b)
^^^^^^^^^^^^^^^ RSpecRails/MinitestAssertions: Use `expect(b).not_to be_empty`.
assert_includes(a, b)
^^^^^^^^^^^^^^^^^^^^^ RSpecRails/MinitestAssertions: Use `expect(a).to include(b)`.
assert_response :ok
^^^^^^^^^^^^^^^^^^^ RSpecRails/MinitestAssertions: Use `expect(response).to have_http_status(:ok)`.
