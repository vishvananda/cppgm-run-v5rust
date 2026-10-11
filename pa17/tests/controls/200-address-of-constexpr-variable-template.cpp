// N3485 5.3.1p3, 3.2p2 + PA17 variable templates: a constexpr variable
// template specialization is an lvalue whose address may be taken;
// expect exit 0
template<class T> constexpr int id = sizeof(T);
int main() { const int* p = &id<long>; return *p == 8 ? 0 : 1; }
