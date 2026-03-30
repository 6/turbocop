:"foo"
^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

:"hello world"
^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

:"bar_baz"
^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "name": 'val' }
  ^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "role": 1, "color": 2 }
  ^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.
             ^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "\\": [ "\\" ] }
  ^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "": data }
  ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

data.merge({"": data})
            ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

:"symbols__\\",
^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

when :"\\"
     ^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "not_allowed(_\\d)?": false }
  ^^^^^^^^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "allowed_\\d": true }
  ^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{ "dependent_schema(_\\d)?": true }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

JSONLogic.apply(block, data.is_a?(Hash) ? data.merge({"": data}) : { "": data })
                                                      ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.
                                                                     ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

options.merge({"": spec})
               ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

options.merge({"": spec})
               ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

{"": DefaultCompletion, flag: self}
 ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

spec[:""].nil? ? spec.merge({"": DefaultCompletion, flag: self}) : spec
                             ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

spec[:""].nil? ? spec.merge({"": DefaultCompletion}) : spec
                             ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

salaryCompNames = {"": ""}.merge(salary_comps.map{|p| [p.id.to_s, p.name]}.to_h)
                   ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.

expect(json[:credit_note][:metadata]).to eq(foo: "bar", bar: "", baz: nil, "": "qux")
                                                                           ^^^ Style/QuotedSymbols: Prefer single-quoted symbols when you don't need string interpolation or special symbols.
