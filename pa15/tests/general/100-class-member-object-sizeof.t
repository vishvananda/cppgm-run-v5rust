struct A { int x; int y; };
struct B { A sub; int z; };

static_assert(sizeof(B) >= sizeof(A) + sizeof(int), "");

int main() { return 0; }
