struct B {};
struct D : B {};

int pick(...) { return 0; }
int pick(B) { return 1; }

int main() { return pick(D()); }
