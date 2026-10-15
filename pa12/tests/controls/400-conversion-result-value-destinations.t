// Conversion results initialize local, argument and return destinations; expect exit 0
int copies;
struct Small { int n; };
struct Large { long a, b, c; };
struct Indirect { int n; Indirect(int v):n(v){} Indirect(const Indirect&o):n(o.n){++copies;} };
struct Source {
  operator Small(){Small s={3};return s;}
  operator Large(){Large s={4,5,6};return s;}
  operator Indirect(){return Indirect(7);}
};
Small small(Source&s){return s;}
Large large(Source&s){return s;}
Indirect indirect(Source&s){return s;}
int take(Small a,Large b,Indirect c){return a.n+b.a+b.b+b.c+c.n;}
int main(){Source s;Small a=s;Large b=s;Indirect c=s;return copies || a.n!=3 || b.c!=6 || c.n!=7 || small(s).n!=3 || large(s).b!=5 || indirect(s).n!=7 || take(s,s,s)!=25;}
