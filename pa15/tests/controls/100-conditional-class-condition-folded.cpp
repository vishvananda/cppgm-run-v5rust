// N3485 5.16p1, 4p4, 12.3: a class-type ?: condition is evaluated (its
// operand's side effects happen) and converted by calling its conversion
// function, which reads K1::value for a K2 object; expect exit 0
int n;
template<bool B> struct K { operator bool() const { return B; } };
K<true> mk() { ++n; return K<true>(); }
struct K1 { static const bool value = true; operator bool() const { return value; } };
struct K2 : K1 { static const bool value = false; };
int main() { K2 a; int r = (mk() ? 0 : 1) + (n == 1 ? 0 : 2) + (a ? 0 : 4); return r; }
