// N3485 14.5.7p2, 14.5.5p?: S<Ptr<T>> and S<T*> declare the same partial
// specialization: redefinition; expect reject
template<class T> using Ptr = T*;
template<class T> struct S { };
template<class T> struct S<Ptr<T> > { };
template<class T> struct S<T*> { };
int main() { return 0; }
