// N3485 focus: 7.3.1 [namespace.def], 7.3.3 [namespace.udecl], 7.3.4 [namespace.udir]
inline namespace N { namespace { int x; } using namespace M; using M::f; using A = B; }
