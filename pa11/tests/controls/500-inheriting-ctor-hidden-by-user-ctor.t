// N3485 12.9p3: an inheriting constructor is not declared when the class has a
// user-declared constructor with the same signature (D(long)); expect exit 0
struct B { B() {} B(int) {} B(long) {} };
struct D : B { using B::B; D(long) : B(0L), c(1) {} const int c; };
int main() { D d(1L); return d.c == 1 ? 0 : 1; }
