template<class T, class U, U V>
struct base {};

struct X : base<int, unsigned int, static_cast<unsigned int>(0)> {};
