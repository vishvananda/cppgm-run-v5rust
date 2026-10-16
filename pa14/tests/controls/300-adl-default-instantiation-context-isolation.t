// expect exit 0
// A demanded default has its own dependence context, including its first use.
namespace N {
struct Base {};
struct Arg : Base {};
int pick(const Base&) { return 1; }
int operator+(const Base&, int) { return 1; }
}
template<class X> N::Arg make(int value = X()) { return N::Arg(); }
template<class X> X copy(X value = X()) { return value; }
template<class T> int invoke(N::Arg fixed) {
  return pick(make<int>()) + pick(make<int>()) + (make<char>() + 0)
       + pick(copy<N::Arg>()) + pick((make<long>(), fixed));
}
namespace N {
int pick(const Arg&) { return 2; }
int operator+(const Arg&, int) { return 2; }
}
int main() { return invoke<char>(N::Arg()) == 5 ? 0 : 1; }
