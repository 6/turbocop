if foo then bar else baz end
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

unless foo then baz else bar end
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `unless/then/else/end` constructs.

if cond then run else dont end
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

c = if asc; -1 else 1 end
    ^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

c = if asc; 1 else -1 end
    ^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

immediacy = if @immediately; ' immediately'; else; ''; end
            ^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

@node['atts'].each { |k, v| if k.nil?; attl << v; else; attd[k] = v; end }
                            ^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

if at; zt ||= tt; else; at = tt; end
^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

if !ENV['WARBLER_SRC']; gem 'warbler' else gem 'warbler', path: '../../../warbler' end
^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.

if !ENV['JRUBY_RACK_SRC']; gem 'jruby-rack' else gem 'jruby-rack', path: '../../target' end
^ Style/OneLineConditional: Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.
