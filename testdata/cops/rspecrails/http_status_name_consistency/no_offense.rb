it { is_expected.to have_http_status :unprocessable_content }
it { is_expected.to have_http_status :content_too_large }
it { is_expected.to have_http_status :ok }
it { is_expected.to have_http_status :not_found }
it { is_expected.to have_http_status 200 }
it { is_expected.to have_http_status "404" }
