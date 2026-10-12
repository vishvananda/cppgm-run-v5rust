template<class T>struct P{}; P<struct C{static_assert(1 > 0,"m");}> p;
