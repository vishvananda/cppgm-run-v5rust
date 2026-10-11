struct S {int value,id; S(int); S(const S&) noexcept; ~S() noexcept;};
extern int choice(); extern void check(int); extern int step_value();
extern S make_value(); extern S& reference_value(S&); extern S&& rvalue_value(S&);
struct E {S a; int n; S b;};
extern "C" void construction_probe(){S x(1),y(2); E e=choice()?throw 99:E{x,step_value(),y}; check(e.a.value==1 && e.b.value==2); step_value();}

static int selected,progress,limit,live,bad,count,trace[1024];
static int next(){if(++progress==limit)throw progress;return progress;}
S::S(int n):value(n),id(next()){++live;trace[count++]=id;}
S::S(const S& x) noexcept:value(x.value),id(100+count){++live;trace[count++]=id;}
S::~S() noexcept{--live;trace[count++]=-id;}
int choice(){return selected;} void check(int n){if(!n)++bad;}
int step_value(){next();return 10;}
S make_value(){return S(5);} S& reference_value(S& x){return x;} S&& rvalue_value(S& x){return static_cast<S&&>(x);}
extern "C" void construction_probe();
int main(){for(selected=0;selected!=2;++selected){int bound=0,normal_caught=0;
for(limit=0;limit<=bound+1;++limit){progress=live=count=0;int caught=0;
try{construction_probe();}catch(int n){caught=n;}
if(limit==0){bound=progress;normal_caught=caught;if(caught!=(selected?99:0))++bad;}
if(live || (limit>0 && limit<=bound ? caught!=limit : caught!=normal_caught))++bad;
}}return bad?10:0;}
