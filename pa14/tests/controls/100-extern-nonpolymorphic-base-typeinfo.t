// expect exit 0
// A nonpolymorphic base's RTTI is emitted even for an extern class instance.
template<class T> struct Base {
    T n;
};
extern template struct Base<int>;
struct Derived : Base<int> {
    virtual int value() { return n; }
};
int main() {
    Derived d;
    d.n = 7;
    return d.value() - 7;
}
