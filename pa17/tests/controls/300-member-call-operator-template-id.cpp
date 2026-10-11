// N3485 14.2p1, 13.5p4, 5.2.5: f.operator()<double>() calls a member
// operator() template with explicit template arguments; expect exit 0
struct F { template<class T> int operator()() const { return sizeof(T); } };
int main() { F f; return f.operator()<double>() == 8 ? 0 : 1; }
