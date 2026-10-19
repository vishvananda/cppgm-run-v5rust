// expect exit 0
namespace std { struct type_info {}; }
struct Result { virtual int value() { return 7; } };
Result result;
struct A {
    int pad;
    Result& ref;
    Result& get() { return result; }
};
struct L : A {};
struct R : A {};
struct Receiver : L, R {};

// These emitted bodies require Receiver's layout even though no Receiver
// object is defined anywhere in the unit. Both naming paths must survive
// typeid's initially unevaluated operand and its evaluated-use replay.
const std::type_info& method(Receiver* p) { return typeid(p->R::get()); }
const std::type_info& field(Receiver* p) { return typeid(p->R::ref); }
const std::type_info& comma(Receiver* p) { return typeid((p->R::get(), result)); }
const std::type_info& nested(Receiver* p) { return typeid((typeid(p->R::get()), result)); }
int main() { return result.value() == 7 ? 0 : 1; }
