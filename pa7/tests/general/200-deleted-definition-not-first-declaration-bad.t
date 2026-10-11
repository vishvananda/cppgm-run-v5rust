// N3485 8.4.3p4: a deleted definition of a function shall be its first
// declaration; must be rejected
void f(int); void f(int) = delete;
int main() { return 0; }
