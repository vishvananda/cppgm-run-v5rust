struct S{int value,id;S(int);S(const S&);~S()noexcept;};
extern int choice();extern void check(int);extern void check_live(int);extern int step_value();extern void handled(int);
extern S make_value();extern S& reference_value(S&);extern S&& rvalue_value(S&);extern const S& reference_const(const S&);
struct E{S a;int n;S b;};
extern "C" void construction_probe(){struct H{S first;H():first(1){S local(2);S& s=reference_value(local);check(&s==&local);step_value();}};H h;}

static int selected,progress,limit,live,bad,count,trace[4096];
static int next(){if(++progress==limit)throw progress;return progress;}
S::S(int n):value(n),id(next()){++live;trace[count++]=id;}
S::S(const S&x):value(x.value),id(next()){++live;trace[count++]=id;}
S::~S()noexcept{--live;trace[count++]=-id;}
int choice(){return selected;}void check(int n){if(!n)++bad;}void check_live(int n){if(live!=n)++bad;}
int step_value(){next();return 10;}void handled(int n){trace[count++]=-10000-n;}
S make_value(){return S(5);}S& reference_value(S&x){next();return x;}S&& rvalue_value(S&x){next();return static_cast<S&&>(x);}const S& reference_const(const S&x){next();return x;}
extern "C" void construction_probe();

// Destruction traces agree with independent Clang and GCC at O0/O2.
static const unsigned long long expected_trace[2][6]={{226540682150ULL,0ULL,13200130ULL,226540682150ULL,226540682150ULL,226540682150ULL},{226540682150ULL,0ULL,13200130ULL,226540682150ULL,226540682150ULL,226540682150ULL}};
static const int expected_count[2][6]={{4,0,2,4,4,4},{4,0,2,4,4,4}};
int main(){for(selected=0;selected!=2;++selected){int bound=0,normal_caught=0;
for(limit=0;limit<=bound+1;++limit){progress=live=count=0;int caught=0;
try{construction_probe();}catch(int n){caught=n;}
if(limit==0){bound=progress;normal_caught=caught;}
if(live || (limit>0 && limit<=bound ? caught!=limit : caught!=normal_caught))++bad;
unsigned long long hash=0;for(int i=0;i<count;++i)hash=hash*131+static_cast<unsigned long long>(trace[i]+100000);
if(limit>=static_cast<int>(sizeof(expected_trace[0])/sizeof(expected_trace[0][0])) || hash!=expected_trace[selected][limit] || count!=expected_count[selected][limit])++bad;
}}return bad?10:0;}
