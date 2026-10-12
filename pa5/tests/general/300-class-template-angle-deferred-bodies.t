template<class T>struct P{}; P<struct C{void f(){1 > 0;} C():m(1 > 0){} int m; static_assert((1 > 0),"m");}> p;
