[:foo, :bar, :baz]
^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.

[:one, :two]
^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.

x = [:alpha, :beta, :gamma, :delta]
    ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.

# Symbol arrays inside block body of non-parenthesized call should still be flagged
# (only direct arguments are ambiguous, not nested arrays in block body)
describe "test" do
  [:admin, :read, :write]
  ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.
end

it "works" do
  x = [:foo, :bar]
      ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.
end

context "scope" do
  let(:roles) do
    [:viewer, :editor]
    ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.
  end
end

# Symbol arrays inside keyword args of ambiguous calls — not truly ambiguous,
# RuboCop only suppresses top-level (bare) arguments, not hash values
resources :posts, only: [:index, :show] do
                        ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.
  member do
    get :preview
  end
end

hooks.register [:pages, :documents], :pre_render, &(method :before_render)
               ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.

hooks.register [:pages, :documents], :post_render, &(method :after_render)
               ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.

in %I[#{1 + 1}]
   ^ Style/SymbolArray: Use `[:"#{1 + 1}"]` for an array of symbols.

@recorder.inverse_of :drop_table, [:musics, :artists], &block
                                  ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.

%I( one  two #{ 1 } )
^ Style/SymbolArray: Use `[ :one,  :two,  :"#{ 1 }" ]` for an array of symbols.

_GET_ [[:f, [:_ROOT_, :_TEMP_]], [:y_prev, [:_ROOT_, :_TEMP_]], :y] do |f:, y_prev:, y:|
            ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.
                                           ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.

_GET_ [[:mouse_offset_x, :_ROOT_], [:mouse_offset_y, :_ROOT_], :x, :y] do |mouse_offset_x:, mouse_offset_y:, x:, y:|
       ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.
                                   ^ Style/SymbolArray: Use `%i` or `%I` for an array of symbols.
