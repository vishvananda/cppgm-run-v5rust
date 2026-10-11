// N3485 5.1.2p3: each lambda-expression has its own unique closure type; three
// nested immediately-called lambdas are valid; expect exit 0
int f() { return [] { return [] { return [] { return 1; }(); }(); }(); }
int main() { return f() == 1 ? 0 : 1; }
