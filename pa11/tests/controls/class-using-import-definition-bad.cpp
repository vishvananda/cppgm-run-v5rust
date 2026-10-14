// expect reject
struct Base { int select(int); };
struct Derived : Base { using Base::select; };
int Derived::select(int) { return 2; }
int main() { return 0; }
