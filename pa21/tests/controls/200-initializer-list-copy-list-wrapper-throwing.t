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
 S(int v)  : value(v) { checkpoint(); ++alive; ++made; record(v); }
 S(const S& s)  : value(s.value) { checkpoint(); ++alive; ++made; record(value); }
 ~S() noexcept { --alive; ++destroyed; record(-value); }
};
unsigned trace_digest;
// Construction/destruction events followed by p, f, status and live counts.
static const unsigned expected_trace[] = {
 3153675942u, 1185921u, 2577959492u, 4161002823u, 3158419626u, 3159605547u, 3160791468u, 3192811335u, 40321314u, 2617094885u, 4200138216u, 3197555019u, 3198740940u, 3199926861u
};
void record(int v){trace_digest=trace_digest*33u+static_cast<unsigned>(v);}
void checkpoint(){++attempts;if(limit&&attempts==limit)throw limit;}
int pick(){return selected;}
void reset(int p,int f){alive=made=destroyed=attempts=0;selected=p;limit=f;trace_digest=0;}
bool trace_matches(int p,int f,int status){
 record(p);record(f);record(status);record(alive);record(made);record(destroyed);
 return trace_digest==expected_trace[p*7+f];
}
using IL = std::initializer_list<S>;
extern void reset(int,int);
IL copy(IL x) { if(alive!=3) throw 77; return x; }
const IL& borrow(const IL& x) { if(alive!=3) throw 77; return x; }
IL local() { IL x={4,5}; return x; }
void stop() { checkpoint(); }
int test(){S earlier(9); {const IL& xs={1,2}; {IL copy=xs; const IL& ref=copy; if(alive!=3||ref.size()!=2)return 2;}if(alive!=3)return 3;}if(alive!=1)return 4;return 0;}
int main() {
 for(int p=0;p<2;++p) for(int f=0;f<7;++f) {
  reset(p,f); int status=0;
  try { status=test(); } catch(int e) { if(e==77) status=77; }
  if(!trace_matches(p,f,status)) return 2;
  if(status||alive||made!=destroyed) return 1;
 }
 return 0;
}
