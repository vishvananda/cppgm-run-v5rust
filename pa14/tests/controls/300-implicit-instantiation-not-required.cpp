// N3485 14.7.1p1: a class template specialization is instantiated only where a
// complete type is required; a function declaration's parameter/return type
// and sizeof of a pointer to it do not require it; expect exit 0
template<class T> struct W { typename T::nope x; };
W<int> g(W<int>&);
int main() { return sizeof(W<int>*) == sizeof(void*) ? 0 : 1; }
