// N3485 14.5.6.2p3 and 14.8.2.4p8: the object precedes the member pack.
// Explicit member calls still compare only the written parameters; expect exit 0
struct A {
    template<class... R> int operator*(R&...) { return 4; }
    template<class... R> int operator+(R...) { return 1; }
};
template<class... R> int operator*(R&...) { return 8; }
template<class T> int operator+(T&) { return 9; }
struct B {
    template<class R> int operator*(R&) { return 2; }
    template<class R> int operator*(R const&) { return 7; }
};
int main() {
    A a;
    B b;
    int n = 0;
    return (a * n) + b.operator*<int>(n) + (+a) - 7;
}
