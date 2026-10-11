// N3485 focus: 7.3.3 [namespace.udecl] using-declaration shall not name a template-id
namespace N { template<class T> void f(T); }
using N::f<int>;
