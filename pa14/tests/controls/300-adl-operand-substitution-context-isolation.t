// expect exit 0
// A signature's own dependent names do not make its fixed caller dependent.
namespace N {
struct Base {};
struct Arg : Base {};
int pick(const Base&) { return 1; }
int operator+(const Base&, int) { return 1; }
}
template<class X> auto make(X) -> decltype(X(), N::Arg()) { return N::Arg(); }
template<class X> auto failed(X) -> decltype(X(), X::missing());
N::Arg failed(...) { return N::Arg(); }

template<class T> int invoke(N::Arg fixed) {
  return pick(make(0)) + pick(make(0)) + pick(make<char>(0))
       + (make(0L) + 0) + pick((make(0.0), fixed))
       + pick(failed(0)) + pick(failed(0));
}
namespace N {
int pick(const Arg&) { return 2; }
int operator+(const Arg&, int) { return 2; }
}
int main() { return invoke<char>(N::Arg()) == 7 ? 0 : 1; }
