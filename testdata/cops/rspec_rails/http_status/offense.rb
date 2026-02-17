it { is_expected.to have_http_status 200 }
                                     ^^^ RSpecRails/HttpStatus: Prefer `:ok` over `200` to describe HTTP status code.
it { is_expected.to have_http_status "200" }
                                     ^^^^^ RSpecRails/HttpStatus: Prefer `:ok` over `"200"` to describe HTTP status code.
it { is_expected.to have_http_status(404) }
                                     ^^^ RSpecRails/HttpStatus: Prefer `:not_found` over `404` to describe HTTP status code.
