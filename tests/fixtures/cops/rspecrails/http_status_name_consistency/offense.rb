it { is_expected.to have_http_status :unprocessable_entity }
                                     ^^^^^^^^^^^^^^^^^^^^^ RSpecRails/HttpStatusNameConsistency: Prefer `:unprocessable_content` over `:unprocessable_entity`.
it { is_expected.to have_http_status :payload_too_large }
                                     ^^^^^^^^^^^^^^^^^^ RSpecRails/HttpStatusNameConsistency: Prefer `:content_too_large` over `:payload_too_large`.
it { is_expected.to have_http_status(:unprocessable_entity) }
                                     ^^^^^^^^^^^^^^^^^^^^^ RSpecRails/HttpStatusNameConsistency: Prefer `:unprocessable_content` over `:unprocessable_entity`.
