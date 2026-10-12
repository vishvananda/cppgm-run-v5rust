// Exact frozen types-mode declaration ownership regression.
struct C{C(volatile C&)=default;};
