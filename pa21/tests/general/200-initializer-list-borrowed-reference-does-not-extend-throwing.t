namespace std {
template<class E> class initializer_list {
 const E* first; unsigned long count;
 initializer_list(const E* p,unsigned long n):first(p),count(n){}
 public:
 initializer_list():first(0),count(0){}
 unsigned long size()const{return count;}
 const E* begin()const{return first;}
 const E* end()const{return first+count;}
};
}
int alive, made, destroyed, selected, limit, attempts;
extern void record(int); extern void checkpoint(); extern int pick();
struct S {
 int value;
 S(int v)  : value(v) { checkpoint(); ++alive; ++made; record(v); }
 S(const S& s)  : value(s.value) { checkpoint(); ++alive; ++made; record(value); }
 ~S() noexcept { --alive; ++destroyed; record(-value); }
};
extern "C" int printf(const char*,...);
void record(int v){printf("%d,",v);}
void checkpoint(){++attempts;if(limit&&attempts==limit)throw limit;}
int pick(){return selected;}
void reset(int p,int f){alive=made=destroyed=attempts=0;selected=p;limit=f;}
void print(int p,int f,int status){printf(":%d:%d:%d:%d:%d:%d\n",p,f,status,alive,made,destroyed);}
using IL = std::initializer_list<S>;
extern void reset(int,int); extern void print(int,int,int);
IL copy(IL x) { if(alive!=3) throw 77; return x; }
const IL& borrow(const IL& x) { if(alive!=3) throw 77; return x; }
IL local() { IL x={4,5}; return x; }
void stop() { checkpoint(); }
int test(){S earlier(9); {const IL& xs=borrow({1,2});(void)xs;if(alive!=1)return 2;}if(alive!=1)return 3;return 0;}
int main() {
 for(int p=0;p<2;++p) for(int f=0;f<7;++f) {
  reset(p,f); int status=0;
  try { status=test(); } catch(int e) { if(e==77) status=77; }
  print(p,f,status);
  if(status||alive||made!=destroyed) return 1;
 }
 return 0;
}
