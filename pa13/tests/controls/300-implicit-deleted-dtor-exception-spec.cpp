// N3485 12.4p5, 15.4p14, 10.3p16: B's implicit destructor is deleted (A's is)
// and noexcept like A's; it overrides A's deleted virtual destructor without
// any exception-specification conflict; expect exit 0
struct A { virtual ~A() = delete; private: static void operator delete(void*); };
struct B : A { };
int main() { return 0; }
