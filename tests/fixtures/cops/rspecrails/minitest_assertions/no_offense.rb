expect(b).to eq(a)
expect(b).not_to eq(a)
expect(a).to eq(nil)
expect(a).not_to be_empty
expect(response).to have_http_status(:ok)
expect(a).to include(b)
