(variable & flags).positive?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/BitwisePredicate: Replace with `anybits?` for comparison with bit flags.

(variable & flags) > 0
^^^^^^^^^^^^^^^^^^^^^^ Style/BitwisePredicate: Replace with `anybits?` for comparison with bit flags.

(variable & flags) == 0
^^^^^^^^^^^^^^^^^^^^^^^ Style/BitwisePredicate: Replace with `nobits?` for comparison with bit flags.

is_found = constant_value.is_a?(Integer) && (integer & constant_value) == constant_value
                                            ^ Style/BitwisePredicate: Replace with `integer.allbits?(constant_value)` for comparison with bit flags.

(@swt_widget.style & comparison) == comparison
^ Style/BitwisePredicate: Replace with `@swt_widget.style.allbits?(comparison)` for comparison with bit flags.

(swt_style & SWT::SWTProxy[style]) == SWT::SWTProxy[style]
^ Style/BitwisePredicate: Replace with `swt_style.allbits?(SWT::SWTProxy[style])` for comparison with bit flags.

if (signatures.options & Regexp::MULTILINE) == Regexp::MULTILINE
   ^ Style/BitwisePredicate: Replace with `signatures.options.allbits?(Regexp::MULTILINE)` for comparison with bit flags.

next unless (clauses.values & partial_clauses) == clauses.values
            ^ Style/BitwisePredicate: Replace with `partial_clauses.allbits?(clauses.values)` for comparison with bit flags.

if (current.to_i(8) & mode_part['fullcontrol']) == mode_part['fullcontrol']
   ^ Style/BitwisePredicate: Replace with `current.to_i(8).allbits?(mode_part['fullcontrol'])` for comparison with bit flags.

(specified_sids & current_sids) == specified_sids
^ Style/BitwisePredicate: Replace with `current_sids.allbits?(specified_sids)` for comparison with bit flags.

(specified_sids & current_sids) == specified_sids
^ Style/BitwisePredicate: Replace with `current_sids.allbits?(specified_sids)` for comparison with bit flags.
