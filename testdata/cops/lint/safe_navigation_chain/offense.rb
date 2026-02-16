x&.foo.bar
      ^^^^ Lint/SafeNavigationChain: Do not chain ordinary method call after safe navigation operator.

x&.foo(x).bar(y)
         ^^^^^^^ Lint/SafeNavigationChain: Do not chain ordinary method call after safe navigation operator.

x&.foo + bar
      ^^^^^^ Lint/SafeNavigationChain: Do not chain ordinary method call after safe navigation operator.

x&.foo[bar]
      ^^^^^ Lint/SafeNavigationChain: Do not chain ordinary method call after safe navigation operator.
