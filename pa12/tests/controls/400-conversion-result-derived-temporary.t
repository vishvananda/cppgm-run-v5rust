// A complete derived result is destroyed after the base transfer; expect exit 0
int alive,copies;
struct A{int n;A():n(2){}A(const A&o):n(o.n+10){if(alive!=1)copies=100;else ++copies;}};
struct D:A{D(){++alive;n=5;}~D(){--alive;}};
struct S{operator D(){return D();}};
A make(S&s){return s;}
int main(){S s;A a=s;if(alive || a.n!=15 || copies!=1)return 1;A b=make(s);return alive || b.n!=15 || copies!=2;}
