// N3485 5.2.9p4 + 4.12: static_cast<bool>(p) for a pointer to member function
// is a boolean conversion (direct-initialization bool t(p)); expect exit 0
struct F { int f(); };
typedef int (F::*FF)();
int main() { FF p = 0; return static_cast<bool>(p) ? 1 : 0; }
