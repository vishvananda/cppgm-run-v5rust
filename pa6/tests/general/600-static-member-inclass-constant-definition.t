// Exact frozen types-mode declaration ownership regression.
struct C{static const int x=2;};const int C::x;int a[C::x];
