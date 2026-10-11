// VALIDATION: compile-fail
struct Base { Base() = default; Base(Base&&) = delete; };
struct Derived : Base { Derived() = default; Derived(Derived&&) = default; };
int main() { Derived source; Derived destination(static_cast<Derived&&>(source)); }
