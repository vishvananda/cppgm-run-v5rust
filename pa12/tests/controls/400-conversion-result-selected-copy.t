// N3485 8.5p16 and 13.3.1.4: consume one converter result with its selected copy.
// expect exit 0
int calls;
int copies;
struct Value {
    int n;
    Value(int v) : n(v) {}
    Value(const Value& other) : n(other.n + 1) { ++copies; }
};
Value original(5);
struct Proxy {
    operator const Value&() { ++calls; return original; }
};
struct Pad { int n; Pad() : n(0) {} };
struct Derived : Pad, Value { Derived() : Value(9) {} };
Derived derived;
struct DerivedProxy {
    operator const Derived&() { ++calls; return derived; }
};
int main() {
    Proxy proxy;
    Value first = proxy;
    DerivedProxy other;
    Value second = other;
    return first.n == 6 && second.n == 10 && calls == 2 && copies == 2 ? 0 : 1;
}
