# -*- coding: utf-8 -*- #
# frozen_string_literal: true
@prefix = opts.fetch(:prefix) { 'RG' }
               ^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFetchBlock: Use `fetch(:prefix, 'RG')` instead of `fetch(:prefix) { 'RG' }`.
