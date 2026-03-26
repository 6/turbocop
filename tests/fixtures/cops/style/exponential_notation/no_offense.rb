x = 120.03
y = 0.07390
z = 1e6
a = 3.1415e3
b = -9.999e3
c = 5.02e-3
2.5e20.round(-20).should eql(3 * 10 ** 20)
-2.5e20.round(-20).should eql(-3 * 10 ** 20)

# Uppercase E is not checked by RuboCop (only lowercase e)
d = 0.22E1
e = 0.11E1
