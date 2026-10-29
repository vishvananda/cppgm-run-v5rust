// expect exit 0
// A user function's direct reference binding does not introduce a fresh
// temporary check; its selected operand-copy binding has no new base access.
struct Temporary { private: ~Temporary(); };
struct ReferenceSource { operator Temporary(); };
inline const Temporary& bind_result(ReferenceSource& value) { return value; }
struct Base { Base(const Base&); };
struct Result : private Base {};
struct CopySource { operator Result&(); };
inline Base copy_result(CopySource& value) { return value; }
int main() { return 0; }
