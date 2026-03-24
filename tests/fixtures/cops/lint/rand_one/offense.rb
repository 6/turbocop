rand 1
^^^^^^ Lint/RandOne: `rand 1` always returns `0`. Perhaps you meant `rand(2)` or `rand`?
rand(1)
^^^^^^^ Lint/RandOne: `rand(1)` always returns `0`. Perhaps you meant `rand(2)` or `rand`?
Kernel.rand(1)
^^^^^^^^^^^^^^ Lint/RandOne: `Kernel.rand(1)` always returns `0`. Perhaps you meant `rand(2)` or `rand`?
x = rand 1
    ^^^^^^ Lint/RandOne: `rand 1` always returns `0`. Perhaps you meant `rand(2)` or `rand`?
y = rand(1)
    ^^^^^^^ Lint/RandOne: `rand(1)` always returns `0`. Perhaps you meant `rand(2)` or `rand`?
