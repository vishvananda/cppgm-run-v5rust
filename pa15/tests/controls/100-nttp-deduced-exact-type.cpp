// N3485 14.8.2.5p17 (example): A<1> (int argument) does not deduce short s in
// f(A<s>); deduction fails and f(...) is chosen; expect exit 0
template<int i> struct A {};
template<short s> int f(A<s>) { return 1; }
int f(...) { return 0; }
int main() { A<1> a; return f(a); }
