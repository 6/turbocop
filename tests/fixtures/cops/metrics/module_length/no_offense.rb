module ShortModule
  def foo
    1
  end
end

module EmptyModule
end

module AnotherShort
  def self.bar
    2
  end

  def self.baz
    3
  end

  def self.qux
    4
  end
end

# Namespace module (body is exactly one class) should not be counted
module Outer
  class Inner
    def a; end
    def b; end
    def c; end
    def d; end
    def e; end
    def f; end
    def g; end
    def h; end
    def i; end
    def j; end
    def k; end
    def l; end
    def m; end
    def n; end
    def o; end
    def p; end
    def q; end
    def r; end
    def s; end
    def t; end
    def u; end
    def v; end
    def w; end
    def x; end
    def y; end
    def z; end
    def aa; end
    def bb; end
    def cc; end
    def dd; end
    def ee; end
    def ff; end
    def gg; end
    def hh; end
    def ii; end
    def jj; end
    def kk; end
    def ll; end
    def mm; end
    def nn; end
    def oo; end
    def pp; end
    def qq; end
    def rr; end
    def ss; end
    def tt; end
    def uu; end
    def vv; end
    def ww; end
    def xx; end
    def yy; end
    def zz; end
    def a1; end
    def a2; end
    def a3; end
    def a4; end
    def a5; end
    def a6; end
    def a7; end
    def a8; end
    def a9; end
    def a10; end
    def a11; end
    def a12; end
    def a13; end
    def a14; end
    def a15; end
    def a16; end
    def a17; end
    def a18; end
    def a19; end
    def a20; end
    def a21; end
    def a22; end
    def a23; end
    def a24; end
    def a25; end
    def a26; end
    def a27; end
    def a28; end
    def a29; end
    def a30; end
    def a31; end
    def a32; end
    def a33; end
    def a34; end
    def a35; end
    def a36; end
    def a37; end
    def a38; end
    def a39; end
    def a40; end
    def a41; end
    def a42; end
    def a43; end
    def a44; end
    def a45; end
    def a46; end
    def a47; end
    def a48; end
    def a49; end
    def a50; end
  end
end

# Namespace module (body is exactly one module) should not be counted
module TopLevel
  module Nested
    def a; end
    def b; end
    def c; end
  end
end

# Module with inner class: inner class lines are excluded from outer count.
# Inner class has 95 body lines (under Max:100).
# Without inner class exclusion, outer module would have >100 body lines.
# With inner class excluded, outer module has only 10 body lines.
module OuterWithInnerClass
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
# Module with inner class inside begin block - inner class lines should be excluded
module OuterWithBeginClass
  CONST_A = 1
  CONST_B = 2
  CONST_C = 3
  CONST_D = 4
  CONST_E = 5
  begin
    class InnerInBegin
      def n01; end
      def n02; end
      def n03; end
      def n04; end
      def n05; end
      def n06; end
      def n07; end
      def n08; end
      def n09; end
      def n10; end
      def n11; end
      def n12; end
      def n13; end
      def n14; end
      def n15; end
      def n16; end
      def n17; end
      def n18; end
      def n19; end
      def n20; end
      def n21; end
      def n22; end
      def n23; end
      def n24; end
      def n25; end
      def n26; end
      def n27; end
      def n28; end
      def n29; end
      def n30; end
      def n31; end
      def n32; end
      def n33; end
      def n34; end
      def n35; end
      def n36; end
      def n37; end
      def n38; end
      def n39; end
      def n40; end
      def n41; end
      def n42; end
      def n43; end
      def n44; end
      def n45; end
      def n46; end
      def n47; end
      def n48; end
      def n49; end
      def n50; end
      def n51; end
      def n52; end
      def n53; end
      def n54; end
      def n55; end
      def n56; end
      def n57; end
      def n58; end
      def n59; end
      def n60; end
      def n61; end
      def n62; end
      def n63; end
      def n64; end
      def n65; end
      def n66; end
      def n67; end
      def n68; end
      def n69; end
      def n70; end
      def n71; end
      def n72; end
      def n73; end
      def n74; end
      def n75; end
      def n76; end
      def n77; end
      def n78; end
      def n79; end
      def n80; end
      def n81; end
      def n82; end
      def n83; end
      def n84; end
      def n85; end
      def n86; end
      def n87; end
      def n88; end
      def n89; end
      def n90; end
    end
  end
  CONST_F = 6
  CONST_G = 7
  CONST_H = 8
  CONST_I = 9
  CONST_J = 10
end

# Module with inner class inside if block - inner class lines should be excluded
module OuterWithIfClass
  CONST_A = 1
  CONST_B = 2
  CONST_C = 3
  CONST_D = 4
  CONST_E = 5
  if defined?(SomeGem)
    class InnerInIf
      def p01; end
      def p02; end
      def p03; end
      def p04; end
      def p05; end
      def p06; end
      def p07; end
      def p08; end
      def p09; end
      def p10; end
      def p11; end
      def p12; end
      def p13; end
      def p14; end
      def p15; end
      def p16; end
      def p17; end
      def p18; end
      def p19; end
      def p20; end
      def p21; end
      def p22; end
      def p23; end
      def p24; end
      def p25; end
      def p26; end
      def p27; end
      def p28; end
      def p29; end
      def p30; end
      def p31; end
      def p32; end
      def p33; end
      def p34; end
      def p35; end
      def p36; end
      def p37; end
      def p38; end
      def p39; end
      def p40; end
      def p41; end
      def p42; end
      def p43; end
      def p44; end
      def p45; end
      def p46; end
      def p47; end
      def p48; end
      def p49; end
      def p50; end
      def p51; end
      def p52; end
      def p53; end
      def p54; end
      def p55; end
      def p56; end
      def p57; end
      def p58; end
      def p59; end
      def p60; end
      def p61; end
      def p62; end
      def p63; end
      def p64; end
      def p65; end
      def p66; end
      def p67; end
      def p68; end
      def p69; end
      def p70; end
      def p71; end
      def p72; end
      def p73; end
      def p74; end
      def p75; end
      def p76; end
      def p77; end
      def p78; end
      def p79; end
      def p80; end
      def p81; end
      def p82; end
      def p83; end
      def p84; end
      def p85; end
      def p86; end
      def p87; end
      def p88; end
      def p89; end
      def p90; end
    end
  end
  CONST_F = 6
  CONST_G = 7
  CONST_H = 8
  CONST_I = 9
  CONST_J = 10
end

