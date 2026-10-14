// N3485 8.5p16, 3.6.2 + PA17 variable templates: a variable template of class
// type initialized by a constructor call; expect exit 0
struct C { int k; C(int x) : k(x) {} };
template<int N> C obj(N * 2);
int main() { return obj<3>.k == 6 ? 0 : 1; }
