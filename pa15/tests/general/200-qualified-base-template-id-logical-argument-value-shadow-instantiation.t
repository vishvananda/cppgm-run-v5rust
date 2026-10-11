// VALIDATION: compile-pass
// N3485 focus: 14.2 [temp.names], 14.7.1 [temp.inst]

namespace m {
template<bool> struct flag { typedef int type; };
}

namespace n {
int flag;
template<class T> struct inner : m::flag<sizeof(T) && true> {};
template<class T> struct outer { typedef typename inner<T>::type type; };
}

n::outer<int>::type value;

int main() { return value; }
