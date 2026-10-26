// Preserve the frozen qualified-static-member rule and accumulated properties.
struct C { static int x; };
mutable int C::x;
extern int C::x;
int main() { C::x = 3; return C::x - 3; }
