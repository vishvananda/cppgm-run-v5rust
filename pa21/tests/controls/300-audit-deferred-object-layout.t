// expect exit 0
namespace std { struct type_info {}; }
struct Result { virtual int value() { return 7; } };
Result result;
struct Plain { int pad; Result& ref; };
const std::type_info& field(Plain* p) { return typeid(p->ref); }
struct Base { int pad; };
struct Derived : Base { int tail; };
const std::type_info& conversion(Derived* p) { return typeid((static_cast<Base*>(p), result)); }
const std::type_info& nested(Plain* p) { return typeid((typeid(p->ref), result)); }
int main() { return result.value() == 7 ? 0 : 1; }
