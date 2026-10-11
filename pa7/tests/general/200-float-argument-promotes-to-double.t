// N3485 4.6, 13.3.3.1.1: float to double is a promotion, so f(double) is the
// best match for a float argument over f(int) and f(long) (full program: exit
// 3)
int f(int) { return 1; } long f(long) { return 2; } double f(double) { return 3; }
int main() { float fl = 1.5f; return static_cast<char>(f(fl)); } // 3
