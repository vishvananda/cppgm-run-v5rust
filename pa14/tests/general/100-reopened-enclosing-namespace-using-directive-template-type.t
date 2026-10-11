namespace n { int x(int); }
namespace n { namespace d { template<class> struct x {}; } }
namespace n { namespace d {
void f() { using namespace n; (void)sizeof(x<int>); }
} }
