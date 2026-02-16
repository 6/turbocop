%w[%w[foo]]
^^^^^^^^^^^ Lint/NestedPercentLiteral: Within percent literals, nested percent literals do not function and may be unwanted in the result.
%w(%i[bar])
^^^^^^^^^^^ Lint/NestedPercentLiteral: Within percent literals, nested percent literals do not function and may be unwanted in the result.
%i[%w(baz)]
^^^^^^^^^^^ Lint/NestedPercentLiteral: Within percent literals, nested percent literals do not function and may be unwanted in the result.
