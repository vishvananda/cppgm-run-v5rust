struct C { typedef int word; };
template<class T> struct P : T {};
#define R1 sizeof(P<P<C>>(1));
#define R2 R1 R1
#define R4 R2 R2
#define R8 R4 R4
#define R16 R8 R8
#define R32 R16 R16
#define R64 R32 R32
#define R128 R64 R64
#define R256 R128 R128

void before_alias() { R256 }
using A = P<C>;
void after_alias() { A::word * p; }
