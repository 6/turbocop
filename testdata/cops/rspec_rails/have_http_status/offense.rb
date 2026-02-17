it { expect(response.status).to be(200) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpecRails/HaveHttpStatus: Prefer `expect(response).to have_http_status(200)` over `expect(response.status).to be(200)`.
it { expect(response.status).not_to eq(404) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpecRails/HaveHttpStatus: Prefer `expect(response).not_to have_http_status(404)` over `expect(response.status).not_to eq(404)`.
it { expect(response.code).to eq("200") }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpecRails/HaveHttpStatus: Prefer `expect(response).to have_http_status(200)` over `expect(response.code).to eq("200")`.
it { expect(last_response.status).to eql(301) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpecRails/HaveHttpStatus: Prefer `expect(last_response).to have_http_status(301)` over `expect(last_response.status).to eql(301)`.
