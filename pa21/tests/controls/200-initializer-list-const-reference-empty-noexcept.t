// expect exit 0
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
 S(int v) noexcept : value(v) {  ++alive; ++made; record(v); }
 S(const S& s) noexcept : value(s.value) {  ++alive; ++made; record(value); }
 ~S() noexcept { --alive; ++destroyed; record(-value); }
};
unsigned trace_digest;
// Construction/destruction events followed by p, f, status and live counts.
static const unsigned expected_trace[] = {
 2575587650u, 2576773571u, 2614723043u, 2615908964u
};
void record(int v){trace_digest=trace_digest*33u+static_cast<unsigned>(v);}
void checkpoint(){++attempts;if(limit&&attempts==limit)throw limit;}
int pick(){return selected;}
void reset(int p,int f){alive=made=destroyed=attempts=0;selected=p;limit=f;trace_digest=0;}
bool trace_matches(int p,int f,int status){
 record(p);record(f);record(status);record(alive);record(made);record(destroyed);
 return trace_digest==expected_trace[p*2+f];
}
using IL = std::initializer_list<S>;
extern void reset(int,int);
IL copy(IL x) { if(alive!=3) throw 77; return x; }
const IL& borrow(const IL& x) { if(alive!=3) throw 77; return x; }
IL local() { IL x={4,5}; return x; }
void stop() { checkpoint(); }
int test(){S earlier(9); {const IL& xs={}; if(alive!=1||xs.size()!=0)return 2;} if(alive!=1)return 3;return 0;}
int main() {
 for(int p=0;p<2;++p) for(int f=0;f<2;++f) {
  reset(p,f); int status=0;
  try { status=test(); } catch(int e) { if(e==77) status=77; }
  if(!trace_matches(p,f,status)) return 2;
  if(status||alive||made!=destroyed) return 1;
 }
 return 0;
}
