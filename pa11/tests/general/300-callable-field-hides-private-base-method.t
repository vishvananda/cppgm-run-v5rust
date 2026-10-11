struct B { private: void add(int, int, int); };
struct F { void operator()(int, int, int) const {} };
struct D : private B { F add; };
int main() { D d; d.add(1, 2, 3); }
