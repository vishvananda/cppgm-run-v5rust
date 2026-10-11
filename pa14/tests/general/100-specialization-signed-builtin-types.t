// HHC-058
template<class A, class B, class C, class D, class E>
struct type_list {};

using signed_types =
    type_list<signed char, signed short, signed int, signed long, signed long long>;

int main() { return 0; }
