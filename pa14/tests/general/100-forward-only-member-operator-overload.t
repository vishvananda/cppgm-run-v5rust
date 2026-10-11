template<class T> struct S;
typedef S<int> Alias;

template<class T>
S<T>& manip(S<T>& x);

template<class T>
struct S {
  S& operator<<(S& (*pf)(S&)) { return pf(*this); }
};

template<class T>
S<T>& manip(S<T>& x) { return x; }

template<class T>
T&& declval();

typedef decltype(declval<Alias&>() << manip<int>) Result;

int main() { return 0; }
