template<class> struct S;
template<> struct S<int> { static_assert(sizeof(int) == 0, "must reject"); };
