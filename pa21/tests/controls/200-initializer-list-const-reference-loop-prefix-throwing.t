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
 2351565082u, 1185921u, 2577959492u, 4161002823u, 3158419626u, 3234977965u, 2388916624u, 3454154643u, 3148635894u, 1074977785u, 1360116444u, 3354420223u, 2686873538u, 2429018725u, 2368167976u, 2390700475u, 40321314u, 2617094885u, 4200138216u, 3197555019u, 3274113358u, 2428052017u, 3493290036u, 3187771287u, 1114113178u, 1399251837u, 3393555616u, 2726008931u, 2468154118u, 2407303369u
};
void record(int v){trace_digest=trace_digest*33u+static_cast<unsigned>(v);}
void checkpoint(){++attempts;if(limit&&attempts==limit)throw limit;}
int pick(){return selected;}
void reset(int p,int f){alive=made=destroyed=attempts=0;selected=p;limit=f;trace_digest=0;}
bool trace_matches(int p,int f,int status){
 record(p);record(f);record(status);record(alive);record(made);record(destroyed);
 return trace_digest==expected_trace[p*15+f];
}
using IL = std::initializer_list<S>;
extern void reset(int,int);
IL copy(IL x) { if(alive!=3) throw 77; return x; }
const IL& borrow(const IL& x) { if(alive!=3) throw 77; return x; }
IL local() { IL x={4,5}; return x; }
void stop() { checkpoint(); }
int test() {
 try {
  S earlier(9);
  {const IL& xs={1,2,3,4,5,6,7,8,10,11,12,13}; if(alive!=13||xs.size()!=12)return 2;}
  if(alive!=1)return 3;
  return 0;
 } catch(int e) { return e==77 ? 77 : 0; }
}
int main() {
 for(int p=0;p<2;++p) for(int f=0;f<15;++f) {
  reset(p,f); int status=0;
  try { status=test(); } catch(int e) { if(e==77) status=77; }
  if(!trace_matches(p,f,status)) return 2;
  if(status||alive||made!=destroyed) return 1;
 }
 return 0;
}
