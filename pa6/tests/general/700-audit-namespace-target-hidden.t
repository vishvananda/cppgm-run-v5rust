// Frozen PA6 types-mode scope/declaration ownership regression.
namespace N{typedef int I;} void f(){int N;using namespace N;I x;}
