// N3485 5.2.8p3/p4: typeid of an expression (or type-id) of incomplete class
// type is ill-formed. Must reject.
#ifdef __GNUC__
#include <typeinfo>
#else
namespace std { class type_info { public: bool operator==(const type_info&) const; }; }
#endif
struct Inc;
int main() { Inc* p = 0; return typeid(*p) == typeid(int); }
