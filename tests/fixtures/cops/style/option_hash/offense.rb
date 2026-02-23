def foo(options = {})
        ^^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `options`.
end

def bar(opts = {})
        ^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `opts`.
end

def baz(params = {})
        ^^^^^^^^^^^ Style/OptionHash: Use keyword arguments instead of an options hash argument `params`.
end
