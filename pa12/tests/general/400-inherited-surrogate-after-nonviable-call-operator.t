typedef int (*binary)(int, int);
int add(int a, int b) { return a + b; }
struct base { operator binary() { return add; } };
struct callable : base { int operator()(int) { return 0; } };
int main() { return callable()(2, 3) == 5 ? 0 : 1; }
