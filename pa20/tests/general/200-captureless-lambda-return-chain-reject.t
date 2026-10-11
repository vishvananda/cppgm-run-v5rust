// A return value is also copy-initialized.
struct Wrapper { Wrapper(int (*)(int)) {} };
Wrapper make() { return [](int n) { return n + 1; }; }
int main() { return 0; }
