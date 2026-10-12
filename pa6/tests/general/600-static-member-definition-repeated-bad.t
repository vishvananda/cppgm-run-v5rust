// Exact frozen types-mode declaration ownership regression.
struct C{static int x;};int C::x=2;int C::x=3;
