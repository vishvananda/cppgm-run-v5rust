// expect reject
// N3485 11p4, 11.2p5: the initial unqualified class name in a
// conversion-function-id's qualifier is access-checked before the receiver.
struct Base { protected: operator int() { return 1; } };
struct Hidden : private Base { public: using Base::operator int; };
struct Derived : Hidden { int get() { return Base::operator int(); } };
int main() {}
