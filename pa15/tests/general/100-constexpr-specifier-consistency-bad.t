// N3485 7.1.5p1: if any declaration of a function has constexpr, all its
// declarations shall (also constexpr on typedef/class declaration/parameter,
// constexpr main 3.6.1p3); must be rejected
constexpr int f();
int f() { return 1; }
int main() { return 0; }
