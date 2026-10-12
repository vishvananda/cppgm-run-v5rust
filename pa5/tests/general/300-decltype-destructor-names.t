struct C{};
template<class T> struct P{};
C::~decltype(C())(){}
P<int>::~decltype(P<int>())();
