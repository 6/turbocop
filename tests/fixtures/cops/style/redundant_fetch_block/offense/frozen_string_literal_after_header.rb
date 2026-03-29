# Copyright 2024 - 2026 Block, Inc.
#
# Use of this source code is governed by an MIT-style
# license that can be found in the LICENSE file or at
# https://opensource.org/licenses/MIT.
#
# frozen_string_literal: true

backend = version_info.fetch("distribution") { "elasticsearch" }.to_sym
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFetchBlock: Use `fetch("distribution", "elasticsearch")` instead of `fetch("distribution") { "elasticsearch" }`.
