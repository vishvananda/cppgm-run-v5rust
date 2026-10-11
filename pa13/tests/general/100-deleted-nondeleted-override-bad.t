// N3485 10.3p16: a deleted function shall not override a function that is not
// deleted (and vice versa); must be rejected
struct A { virtual void f(); };
struct B : A { void f() = delete; };
int main() { return 0; }
