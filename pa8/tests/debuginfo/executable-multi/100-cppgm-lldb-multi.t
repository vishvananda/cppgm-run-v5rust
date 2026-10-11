int add1(int x) { return x + 1; }
int add2(int x) { return add1(x) + 1; }
int main() { return add2(40); }
