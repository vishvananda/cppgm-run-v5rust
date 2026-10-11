// N3485 14.2p1, 13.5p4, 5.2.5: s.operator+<double>(1) calls a member operator+
// template with explicit template arguments; expect exit 0
struct S { template<class T> int operator+(int) const { return sizeof(T); } };
int main() { S s; return s.operator+<double>(1) == 8 ? 0 : 1; }
