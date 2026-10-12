// Exact frozen types-mode declaration ownership regression.
struct C{C()noexcept;};C::C(){}
