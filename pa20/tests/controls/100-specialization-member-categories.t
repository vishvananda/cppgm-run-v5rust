// A member the specializations of a class template declare with different
// categories (a type in one, a value in another) is looked up in the
// specialization the template-id selects (14.5.5.1), wherever the name
// appears: a condition's declaration, a for-range-declaration, a namespace-
// scope declaration, a member a specialization inherits from its base, and a
// primary template defined after its specializations. expect exit 0
template<int> struct pick;
template<> struct pick<2> { typedef int name; };
template<> struct pick<1> { static const int name = 3; };
struct B { typedef int m; };
template<int> struct A;
template<> struct A<1> : B {};
template<> struct A<2> { static const int m = 3; };
template<int> struct P;
template<> struct P<1> { static const int m = 3; };
template<int> struct P { typedef int m; };
int g = 5;
int* gq = &g;
pick<2>::name* gp = &g;
int cond() { if (pick<2>::name* q = gq) return *q - 5 + pick<1>::name - 3; return 1; }
int range() { int arr[2] = { 1, 2 }, s = 0; for (A<1>::m v : arr) s += v; return s - 3; }
int inherited() { int t = 0; { A<1>::m* p = &t; *p = 4; } return t - 4 + A<2>::m - 3; }
int late() { int t = 0; { P<2>::m* p = &t; *p = 4; } return t - 4 + P<1>::m - 3; }
int main() { return cond() + range() + inherited() + late() + *gp - 5; }
