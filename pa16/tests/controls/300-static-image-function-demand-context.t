// N3485 6.7 and 14.7.1: local static images define referred-to template storage on demand.
// expect exit 0
template<int N> struct Value { int n; Value() : n(N) {} };
template<class T> struct Storage { static T value; };
template<class T> T Storage<T>::value;
template<class T> struct Pointer { T* p; constexpr Pointer(T* p) : p(p) {} };
template<class T> struct Binding { T* const& p; };
int first() {
    static const Pointer<Value<1>>& r = Pointer<Value<1>>(&Storage<Value<1>>::value);
    return r.p->n;
}
int second() {
    static constexpr Binding<Value<2>> r = { &Storage<Value<2>>::value };
    return r.p->n;
}
template<int N> int third() {
    static constexpr Binding<Value<N>> r = { &Storage<Value<N>>::value };
    return r.p->n;
}
int main() { return first() == 1 && second() == 2 && third<3>() == 3 ? 0 : 1; }
