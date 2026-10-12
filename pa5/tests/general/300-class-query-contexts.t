struct Root { typedef int word; };
struct Left : Root { int word; };
struct Right : Root {};
struct Choice : Left, Right { void f(){word *p;} };
template<class T> struct Dependent : Choice { void f(){word *p;} };
void g(){ Choice::word *p; Dependent<int>::word *q; }
