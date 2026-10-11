// N3485 5.19p2: a reinterpret_cast (and a conversion from void* to an object
// pointer) is not a core constant expression; must be rejected
constexpr long v = 1;
constexpr const char* p = reinterpret_cast<const char*>(&v);
int main() { return 0; }
