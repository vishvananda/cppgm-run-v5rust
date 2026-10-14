// N3485 14.7.1/1,/6 + 5.3.5: delete v needs V<int, A<int>> complete, so the
// specialization (named with a defaulted template argument) is implicitly
// instantiated and its destructor runs; it is not treated as incomplete
// (std::vector<int>* v; delete v; hits this); expect exit 0
template<class T> struct A {};
int dtors;
template<class T, class Al = A<T> > struct V { T* p; ~V() { ++dtors; } };
void f(V<int>* v) { delete v; }
int main() { f(new V<int>()); return dtors == 1 ? 0 : 1; }
