template<class T> struct Elem { T data; };
static_assert(sizeof(Elem<int>[2]) == sizeof(int) * 2, "");
