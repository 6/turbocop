it { is_expected.to have_http_status :ok }
it { is_expected.to have_http_status :not_found }
it { is_expected.to have_http_status 550 }
it { is_expected.to have_http_status :created }
it { is_expected.to have_http_status :moved_permanently }
