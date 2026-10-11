// N3485 14.6p8, p10: a template argument in a function template's
// template-head that names nothing dependent (a class member's constexpr call)
// is evaluated where the template is defined: enable_if<false, int>::type is
// that definition's error, not a deduction failure.
template<bool B, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
struct K { static constexpr bool no() { return false; } };
template<class T, typename enable_if<K::no(), int>::type = 0> int f(T) { return 1; }
int f(...) { return 0; }
int main() { return f(0); }
