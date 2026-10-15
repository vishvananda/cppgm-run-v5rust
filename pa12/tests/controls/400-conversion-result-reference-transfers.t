// Reference results preserve cv/category for the final constructor; expect exit 0
int copies,moves;
struct A { int n; A(int v):n(v){} A(const A&o):n(o.n+1){++copies;} A(A&&o):n(o.n+2){++moves;} };
A stored(4);
struct L { operator A&(){return stored;} };
struct R { operator A&&(){return static_cast<A&&>(stored);} };
A from_l(L&s){return s;}
A from_r(R&s){return s;}
int main(){L l;R r;A a=l;A b=r;return a.n!=5 || b.n!=6 || from_l(l).n!=5 || from_r(r).n!=6 || copies!=2 || moves!=2;}
