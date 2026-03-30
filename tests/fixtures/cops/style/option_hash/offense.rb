def foo(options = {})
        ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.
end

def bar(opts = {})
        ^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `opts`.
end

def baz(params = {})
        ^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `params`.
end

get_highlight_color = lambda do |opts = {}|
                                 ^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `opts`.

def as_json(options = {})
            ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.
end

def as_json(options = {})
            ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.
end

def initialize(ingredient, options = {})
                           ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.
end

def remove_observer(observer, attribute_or_event, options = {})
                                                  ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.
end

def initialize(param_description, argument, options = {})
                                            ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.
end

RSpec::Matchers.define :have_field do |name, type, opts = {}|
                                                   ^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `opts`.

def to_json( options = {} )
             ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.

def process(opts = {})
            ^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `opts`.
  super(opts)
end
