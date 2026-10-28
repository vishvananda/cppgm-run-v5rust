int calls;
struct S {
  static const int n=3;
  volatile int value;
  volatile int& operator*(){return value;}
};
S& receiver(S& s){++calls;return s;}
void folded(S& s){s.n;receiver(s).n;}
void wrapped(bool b,S& a,S& c){b?*a:*c;}
void direct(S& a){*a;}
struct Empty {};
int operator,(Empty&,Empty&){++calls;return 7;}
void comma(Empty& a,Empty& b){((a,b),0);(0,(a,b));(a,b);}
int main(){
  S a={1},b={2};
  folded(a);wrapped(true,a,b);direct(b);
  Empty x,y;comma(x,y);
  return a.value+b.value==3?0:1;
}
