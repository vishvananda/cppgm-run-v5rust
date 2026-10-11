// N3485 5.3.7/12.8p7: in an unevaluated noexcept operand, E(e) uses E's
// implicitly-declared copy constructor (not yet odr-used/defined); it is
// potentially throwing because D(const D&) is; expect exit 0
struct D { D() {} D(const D&) {} };
struct E { D d; };
extern E e;
int main() { return noexcept(E(e)) ? 1 : 0; }
E e;
