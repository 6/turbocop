# frozen_string_literal: true
hash.fetch(:key) { 5 }
     ^^^^^^^^^^^^^^^^^ Style/RedundantFetchBlock: Use `fetch(:key, 5)` instead of `fetch(:key) { 5 }`.

hash.fetch(:key) { :value }
     ^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFetchBlock: Use `fetch(:key, :value)` instead of `fetch(:key) { :value }`.

hash.fetch(:key) { 'default' }
     ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFetchBlock: Use `fetch(:key, 'default')` instead of `fetch(:key) { 'default' }`.
