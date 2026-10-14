// N3485 15.3p3 + Itanium 2.9.5: a handler for a private base does not match (P
// : private A must use __vmi_class_type_info without the public flag);
// expect exit 0
struct A { virtual ~A() {} };
struct P : private A {};
int main() { try { throw P(); } catch (A&) { return 1; } catch (...) { return 0; } }
