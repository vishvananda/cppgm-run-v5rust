// expect exit 0
namespace std { struct type_info {}; }
struct Result { virtual int get() { return 1; } };
struct Holder { int pad; Result value; };
struct Prefix { int pad; };
struct Base { Result value; };
struct Derived : Prefix, Base {};
struct Receiver { int pad; };

// No complete Holder, Derived or Receiver object otherwise demands layout.
const std::type_info& address(Holder* p) { return typeid(p->*(&Holder::value)); }
const std::type_info& conversion(Derived* p) {
    return typeid(p->*static_cast<Result Derived::*>(&Base::value));
}
const std::type_info& indirect(Receiver* p, Result& (Receiver::*f)()) {
    return typeid((p->*f)());
}
int main() { return 0; }
