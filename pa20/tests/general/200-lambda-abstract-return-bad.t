// N3485 10.4p3, 5.1.2p4: a lambda whose deduced return type is abstract class
// A is ill-formed; expect reject
struct A { virtual int z() const = 0; };
struct B : A { int z() const { return 3; } };
int main() { B b; const A& a = b; auto copy = [&] { return a; }; (void)copy; return 0; }
