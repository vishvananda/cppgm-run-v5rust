// Exact frozen types-mode declaration ownership regression.
struct C{C():x(later){} int x;static const int later=2;};
