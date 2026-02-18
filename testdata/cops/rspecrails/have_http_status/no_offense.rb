it { is_expected.to be(200) }
it { expect(res.status).to be(200) }
it { expect(response.status).to eq("404 Not Found") }
it { expect(response).to have_http_status(200) }
it { expect(last_response).to have_http_status(:ok) }
