// C++11 nested handlers: preserve every original exception and destruction trace.
struct S {int value,id;S();S(int);S(const S&);~S()noexcept;};
static int progress,limit,live,bad,count,seen,trace[4096];
static int next(){if(++progress==limit)throw progress;trace[count++]=progress;++live;return progress;}
S::S():value(7),id(next()){}S::S(int n):value(n),id(next()){}S::S(const S&x):value(x.value),id(next()){}
S::~S()noexcept{trace[count++]=-id;--live;}
int step_value(){if(++progress==limit)throw progress;return 10;}
void handled(int n){if(n!=limit)++bad;++seen;trace[count++]=-10000;}
void handler_fail(){throw 99;}void check(int n){if(!n)++bad;}void mark(int n){trace[count++]=-n;}


// 200-inner-handler-array-three
namespace case_0 {
struct H{S first,values[3];H():first(1){try{step_value();}catch(int n){handled(n);throw;}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={11375981196231759052ULL,0ULL,13200130ULL,226540682150ULL,3887664670782020ULL,14514007865772930832ULL,11375981196231759052ULL};
static const int expected_count[]={8,0,2,4,6,9,8};
}

// 200-inner-handler-base-custom
namespace case_1 {
struct B{S base;B():base(2){}~B()noexcept{mark(20000);}};struct H:B{S first;H():first(1){try{step_value();}catch(int n){handled(n);throw;}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={29676826841780ULL,0ULL,1726697160ULL,3887642180525170ULL,29676826841780ULL};
static const int expected_count[]={5,0,3,6,5};
}

// 200-inner-handler-delegating-rethrow
namespace case_2 {
struct H{S first;H(int):first(1){}H():H(0){try{step_value();}catch(int n){handled(n);throw;}}~H()noexcept{mark(30000);}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={1725387160ULL,0ULL,226365108090ULL,1725387160ULL};
static const int expected_count[]={3,0,4,3};
}

// 200-inner-handler-destructor-miss
namespace case_3 {
struct H{S first;H():first(1){}~H()noexcept(false){mark(30000);try{step_value();}catch(long){mark(90000);}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={1725387160ULL,0ULL,1725387160ULL,1725387160ULL};
static const int expected_count[]={3,0,3,3};
}

// 200-inner-handler-destructor-nine-members
namespace case_4 {
struct H{S first,a,b,c,d,e,f,g,h;H():first(1){}~H()noexcept(false){mark(30000);try{step_value();}catch(int n){handled(n);throw;}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={7304515101698000056ULL,0ULL,13200130ULL,226540682150ULL,3887664670782020ULL,11375981196231759052ULL,1320776811667415494ULL,13249198024493185266ULL,13375186485847794696ULL,38393207279713688ULL,4524784231024410018ULL,7304515101698000056ULL};
static const int expected_count[]={19,0,2,4,6,8,10,12,14,16,20,19};
}

// 200-inner-handler-member-body-handler-locals
namespace case_5 {
struct H{S first;H():first(1){S outside(3);try{S inside(4);step_value();}catch(int n){S handler(5);handled(n);throw;}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={3887664670782020ULL,0ULL,13200130ULL,509284049692844360ULL,14514010519716260922ULL,3887664670782020ULL};
static const int expected_count[]={6,0,2,7,9,6};
}

// 200-inner-handler-member-miss
namespace case_6 {
struct H{S first;H():first(1){try{step_value();}catch(long){mark(90000);}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={13200130ULL,0ULL,13200130ULL,13200130ULL};
static const int expected_count[]={2,0,2,2};
}

// 200-inner-handler-member-replace
namespace case_7 {
struct H{S first;H():first(1){try{step_value();}catch(int n){handled(n);throw 99;}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={13200130ULL,0ULL,1728007160ULL,13200130ULL};
static const int expected_count[]={2,0,3,2};
}

// 200-inner-handler-nested-outer-miss
namespace case_8 {
struct H{S first;H():first(1){try{try{step_value();}catch(int n){handled(n);throw;}}catch(long){mark(90000);}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={13200130ULL,0ULL,1728007160ULL,13200130ULL};
static const int expected_count[]={2,0,3,2};
}

// 200-inner-handler-nested-outer-swallow
namespace case_9 {
struct H{S first;H():first(1){try{try{step_value();}catch(int){throw;}}catch(int n){handled(n);}}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={13200130ULL,0ULL,1728007160ULL,13200130ULL};
static const int expected_count[]={2,0,3,2};
}

// 200-inner-handler-ordinary-throwing-local
namespace case_10 {
struct H{S first;H():first(1){S local(2);step_value();}};
void probe(){H h;check(h.first.value==1);}
static const unsigned long long expected_trace[]={226540682150ULL,0ULL,13200130ULL,226540682150ULL,226540682150ULL};
static const int expected_count[]={4,0,2,4,4};
}

int run(void (*probe)(), const unsigned long long* expected_trace,
        const int* expected_count, unsigned size, int replacement) {
  bad = 0;
  int bound = 0;
  for (limit = 0; limit <= bound + 1; ++limit) {
    progress = live = count = seen = 0;
    int caught = 0;
    try { probe(); } catch (int n) { caught = n; }
    if (limit == 0) bound = progress;
    int expected = limit > 0 && limit <= bound ? (seen && replacement >= 0 ? replacement : limit) : 0;
    if (live || caught != expected) ++bad;
    unsigned long long hash = 0;
    for (int i = 0; i < count; ++i) hash = hash * 131 + static_cast<unsigned long long>(trace[i] + 100000);
    if (static_cast<unsigned>(limit) >= size || hash != expected_trace[limit] || count != expected_count[limit]) ++bad;
  }
  return bad;
}
int main() {
  if (run(case_0::probe, case_0::expected_trace, case_0::expected_count, sizeof(case_0::expected_count) / sizeof(int), -1)) return 1;
  if (run(case_1::probe, case_1::expected_trace, case_1::expected_count, sizeof(case_1::expected_count) / sizeof(int), -1)) return 2;
  if (run(case_2::probe, case_2::expected_trace, case_2::expected_count, sizeof(case_2::expected_count) / sizeof(int), -1)) return 3;
  if (run(case_3::probe, case_3::expected_trace, case_3::expected_count, sizeof(case_3::expected_count) / sizeof(int), -1)) return 4;
  if (run(case_4::probe, case_4::expected_trace, case_4::expected_count, sizeof(case_4::expected_count) / sizeof(int), -1)) return 5;
  if (run(case_5::probe, case_5::expected_trace, case_5::expected_count, sizeof(case_5::expected_count) / sizeof(int), -1)) return 6;
  if (run(case_6::probe, case_6::expected_trace, case_6::expected_count, sizeof(case_6::expected_count) / sizeof(int), -1)) return 7;
  if (run(case_7::probe, case_7::expected_trace, case_7::expected_count, sizeof(case_7::expected_count) / sizeof(int), 99)) return 8;
  if (run(case_8::probe, case_8::expected_trace, case_8::expected_count, sizeof(case_8::expected_count) / sizeof(int), -1)) return 9;
  if (run(case_9::probe, case_9::expected_trace, case_9::expected_count, sizeof(case_9::expected_count) / sizeof(int), 0)) return 10;
  if (run(case_10::probe, case_10::expected_trace, case_10::expected_count, sizeof(case_10::expected_count) / sizeof(int), -1)) return 11;
  return 0;
}
