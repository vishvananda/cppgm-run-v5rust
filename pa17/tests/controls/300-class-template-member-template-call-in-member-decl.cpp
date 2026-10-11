// N3485 3.4.1p7, 14.6.3: a non-dependent call f1<char>() of a member function
// template in a class template's member declaration finds the member;
// expect exit 0
template<class E> struct T { template<class U> static constexpr int f1() { return sizeof(U); } typedef char c[f1<char>()]; };
int main() { return sizeof(T<int>::c) == 1 ? 0 : 1; }
