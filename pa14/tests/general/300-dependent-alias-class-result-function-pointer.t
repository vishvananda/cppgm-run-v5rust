template<class T> struct Identity { typedef T type; };

template<class T> struct Value {
    int number;
    Value(int x) : number(x) {}
};

template<class T>
typename Identity<Value<T> >::type make_alias(int x) { return Value<T>(x); }

template<class T>
Value<T> make_plain(int x) { return Value<T>(x); }

int main() {
    Value<int> (*alias)(int) = make_alias<int>;
    Value<int> (*plain)(int) = make_plain<int>;
    Value<int> a = alias(7);
    Value<int> b = plain(9);
    Value<int> c = make_alias<int>(11);
    return a.number != 7 || b.number != 9 || c.number != 11;
}
