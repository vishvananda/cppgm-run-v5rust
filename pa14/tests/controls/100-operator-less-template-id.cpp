// N3485 14.2p1, 13.5p1: operator< <int>(a, a) is a template-id naming the
// operator function template specialization (the space separates < from
// <int>); expect exit 0
template<class T> struct V { T x; };
template<class T> bool operator<(V<T> a, V<T> b) { return a.x < b.x; }
int main() { V<int> a = { 1 }; return operator< <int>(a, a) ? 1 : 0; }
