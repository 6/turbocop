class ShortClass
  def foo
    1
  end
end

class EmptyClass
end

class AnotherShort
  attr_reader :name
  attr_writer :age

  def initialize(name, age)
    @name = name
    @age = age
  end

  def greet
    "Hello, #{@name}"
  end
end

# Class with inner class: inner class lines are excluded from outer count.
# Inner class has 95 body lines (under Max:100).
# Outer class: 97 body lines (inner class lines) + 10 non-inner = 107 total.
# With inner class excluded, outer has only 10 body lines.
class OuterWithInnerClass
  CONST_A = 1
  CONST_B = 2
  CONST_C = 3
  CONST_D = 4
  CONST_E = 5
  class LargeInner
    def m01; end
    def m02; end
    def m03; end
    def m04; end
    def m05; end
    def m06; end
    def m07; end
    def m08; end
    def m09; end
    def m10; end
    def m11; end
    def m12; end
    def m13; end
    def m14; end
    def m15; end
    def m16; end
    def m17; end
    def m18; end
    def m19; end
    def m20; end
    def m21; end
    def m22; end
    def m23; end
    def m24; end
    def m25; end
    def m26; end
    def m27; end
    def m28; end
    def m29; end
    def m30; end
    def m31; end
    def m32; end
    def m33; end
    def m34; end
    def m35; end
    def m36; end
    def m37; end
    def m38; end
    def m39; end
    def m40; end
    def m41; end
    def m42; end
    def m43; end
    def m44; end
    def m45; end
    def m46; end
    def m47; end
    def m48; end
    def m49; end
    def m50; end
    def m51; end
    def m52; end
    def m53; end
    def m54; end
    def m55; end
    def m56; end
    def m57; end
    def m58; end
    def m59; end
    def m60; end
    def m61; end
    def m62; end
    def m63; end
    def m64; end
    def m65; end
    def m66; end
    def m67; end
    def m68; end
    def m69; end
    def m70; end
    def m71; end
    def m72; end
    def m73; end
    def m74; end
    def m75; end
    def m76; end
    def m77; end
    def m78; end
    def m79; end
    def m80; end
    def m81; end
    def m82; end
    def m83; end
    def m84; end
    def m85; end
    def m86; end
    def m87; end
    def m88; end
    def m89; end
    def m90; end
    def m91; end
    def m92; end
    def m93; end
    def m94; end
    def m95; end
  end
  CONST_F = 6
  CONST_G = 7
  CONST_H = 8
  CONST_I = 9
  CONST_J = 10
end
