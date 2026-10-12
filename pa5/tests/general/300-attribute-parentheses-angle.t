template<class T> struct P{}; P<int alignas((1 > 0))> x; P<int __attribute__((vector_size((1 >> 2))))> y; template<class T=int [[gnu::aligned((1 > 0))]]> struct Q{};
