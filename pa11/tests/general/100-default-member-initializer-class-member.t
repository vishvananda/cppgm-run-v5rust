struct Y { int v; Y() : v(3) {} };
struct X { Y y = Y(); };
int main() { X x; return x.y.v == 3 ? 0 : 1; }
