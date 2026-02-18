def choose_move(who_to_move) # rubocop:disable Metrics/CyclomaticComplexity # rubocop:disable Metrics/AbcSize
                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleCopDisableDirective: More than one disable comment on one line.
end

def foo # rubocop:disable Metrics/MethodLength # rubocop:disable Metrics/AbcSize
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleCopDisableDirective: More than one disable comment on one line.
end

def bar # rubocop:todo Metrics/CyclomaticComplexity # rubocop:todo Metrics/AbcSize
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleCopDisableDirective: More than one disable comment on one line.
end
