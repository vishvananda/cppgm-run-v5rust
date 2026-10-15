// Completed locals and extended results protect construction and the completed return result; expect exit 0
int live;
struct A{int n;A():n(1){++live;}A(const A&o):n(o.n+1){++live;}~A()noexcept(false){--live;}};
struct D:A{int m;};
A stored;
struct Own{operator A(){return A();}};
struct Ref{operator const A&(){return stored;}};
struct Derived{operator D(){return D();}};
void take(A){}
A f(Own&o,Ref&r,Derived&d){A a=o;const A&b=o;take(r);return d;}
int main(){Own o;Ref r;Derived d;{A result=f(o,r,d);if(live!=2 || result.n!=2)return 1;}return live!=1;}
