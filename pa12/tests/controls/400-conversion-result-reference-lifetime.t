// The bound result persists while its receiver ends with the full expression; expect exit 0
int sources,results;
struct A{int n;A():n(9){++results;}~A(){--results;}};
struct S{S(){++sources;}~S(){--sources;}operator A(){return A();}};
int main(){if(sources || results)return 1;{const A&r=S();if(sources || results!=1 || r.n!=9)return 2;}return sources || results;}
