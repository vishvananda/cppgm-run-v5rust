// Exact frozen types-mode declaration ownership regression.
struct C{static int x[];};int C::x[3];int n[sizeof(C)];
