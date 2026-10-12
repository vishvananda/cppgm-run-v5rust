// Exact frozen types-mode declaration ownership regression.
struct C{C();};C::C()=delete;
