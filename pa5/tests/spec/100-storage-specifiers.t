// N3485 focus: 7.1.1 [dcl.stc], 7.1.5 [dcl.constexpr], 7.1.2 [dcl.fct.spec]
extern int ex;
static int internal = 1;
thread_local int tls;
constexpr int cx = 2;
inline int f() { return internal; }
