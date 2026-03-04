obj.print
something.p
nothing.pp

p(class: 'this p method is a DSL')

p(&:this_p_method_is_a_dsl)

# p() used as a method argument (parent is a call node) - not a standalone print
expect(p("abc/").normalized_pattern).to eq("abc")
expect(p("abc/").match?("abc")).to be_truthy
expect(p).to receive(:details).and_return({})
expect(variants[p.id]).to include v1
result = p.expression

# p/puts/print/pp used as argument to another method
do_something(p("value"))
do_something(puts("value"))
do_something(print("value"))
do_something(pp("value"))
