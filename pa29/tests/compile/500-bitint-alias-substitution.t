template<int N> using Bits = unsigned _BitInt(N);
template<int N> Bits<N> select(int);
template<int N> long select(...);
static_assert(__is_same(decltype(select<17>(0)), long), "unsupported type is substitution failure");
int main(){return 0;}
