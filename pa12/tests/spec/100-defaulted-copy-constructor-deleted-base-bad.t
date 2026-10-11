// VALIDATION: compile-fail
struct Base { Base() = default; Base(Base const&) = delete; };
struct Derived : Base { Derived() = default; Derived(Derived const&) = default; };
int main() { Derived source; Derived destination(source); }
