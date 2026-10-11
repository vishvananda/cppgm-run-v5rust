struct S {int value,id;S();S(int);S(const S&);~S()noexcept;};
extern int step_value();extern void handled(int);extern void handler_fail();extern void check(int);extern void mark(int);
struct H{S first;H()try:first(1){S body(3);try{step_value();}catch(int n){handled(n);throw;}}catch(int){mark(40000);throw;}};
extern "C" void construction_probe(){H h;check(h.first.value==1);}

static int progress,limit,live,bad,count,seen,trace[4096];
static int next(){if(++progress==limit)throw progress;trace[count++]=progress;++live;return progress;}
S::S():value(7),id(next()){}S::S(int n):value(n),id(next()){}S::S(const S&x):value(x.value),id(next()){}
S::~S()noexcept{trace[count++]=-id;--live;}
int step_value(){if(++progress==limit)throw progress;return 10;}
void handled(int n){if(n!=limit)++bad;++seen;trace[count++]=-10000;}
void handler_fail(){throw 99;}void check(int n){if(!n)++bad;}void mark(int n){trace[count++]=-n;}
extern "C" void construction_probe();

// Destruction traces agree with independent Clang and GCC runs at O0/O2.
static const unsigned long long expected_trace[]={226540682150ULL,60000ULL,1729277030ULL,3887642183105040ULL,226540682150ULL};
static const int expected_count[]={4,1,3,6,4};
static int template_drops,template_trace;
struct TemplateGuard{int id;template<class T>TemplateGuard(T n)try:id(n){}catch(...){throw;}
 ~TemplateGuard()noexcept(false){
 ++template_drops;template_trace=template_trace*10+id;if(id==2)throw 7;
}};
template<class T>int template_probe(T);
template<class T>int template_probe(T)try{TemplateGuard first(1),second(2);return 8;}
catch(int n){return n;}
int main(){struct Local{int get()try{throw 1;}catch(int){return 7;}};int bound=0;for(limit=0;limit<=bound+1;++limit){progress=live=count=seen=0;int caught=0;
try{construction_probe();}catch(int n){caught=n;}
if(limit==0)bound=progress;int expected=limit>0 && limit<=bound?(seen?limit:limit):0;
if(live || caught!=expected)++bad;
unsigned long long hash=0;for(int i=0;i<count;++i)hash=hash*131+static_cast<unsigned long long>(trace[i]+100000);
if(limit>=static_cast<int>(sizeof(expected_trace)/sizeof(expected_trace[0])) || hash!=expected_trace[limit] || count!=expected_count[limit])++bad;
}return bad||Local().get()!=7||template_probe(0)!=7||template_drops!=2||template_trace!=21?10:0;}
