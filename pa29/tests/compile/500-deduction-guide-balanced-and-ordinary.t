// Guides consume balanced parameters, validate a target type-id, and introduce
// no callable. Adjacent ordinary declarations still use their own grammar.
template<class T> struct Wrap { T value; };
template<class T> explicit Wrap(T (*)(int, int), int = (1 + (2))) -> Wrap<T>;
Wrap(const char (&)[4]) -> Wrap<int>;
template<class T> Wrap<T> make(T x) { return Wrap<T>{x}; }
int main() { return make(6).value - 6; }
