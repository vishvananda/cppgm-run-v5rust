// Exact frozen types-mode declaration ownership regression.
struct C{static int x;};static int C::x;
