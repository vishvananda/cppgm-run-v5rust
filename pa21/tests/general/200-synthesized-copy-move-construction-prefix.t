// C++11 synthesized copy/move construction: one fault-progress harness.
struct S {int value,id; S(); S(int); S(const S&); S(S&&); S& operator=(const S&); S& operator=(S&&); ~S() noexcept;};
static int progress,limit,live,bad,count,marks,trace[4096];
static int next(){if(++progress==limit)throw progress;trace[count++]=progress;++live;return progress;}
S::S():value(7),id(next()){} S::S(int n):value(n),id(next()){}
S::S(const S& x):value(x.value),id(next()){}
S::S(S&& x):value(x.value),id(next()){x.value=-1;}
S& S::operator=(const S& x){if(++progress==limit)throw progress;value=x.value;return *this;}
S& S::operator=(S&& x){if(++progress==limit)throw progress;value=x.value;x.value=-1;return *this;}
S::~S()noexcept{trace[count++]=-id;--live;}
void check(int n){if(!n)++bad;} void mark(int n){trace[count++]=-n;++marks;}


// 200-synthesized-copy-array-3-construction-prefix
namespace case_0 {
struct E{S a[3];S b;};
void probe(){E original;E copy(original);for(int i=0;i<3;++i)check(copy.a[i].value==7);check(copy.b.value==7);}
}

// 200-synthesized-copy-array-after-member-construction-prefix
namespace case_1 {
struct E{S a;S b[12];};
void probe(){E original;E copy(original);check(copy.a.value==7);for(int i=0;i<12;++i)check(copy.b[i].value==7);}
}

// 200-synthesized-copy-array-only-3-construction-prefix
namespace case_2 {
struct E{S a[1][3];};
void probe(){E original;E copy(original);for(int i=0;i<3;++i)check(copy.a[0][i].value==7);}
}

// 200-synthesized-copy-base-custom-destructor-construction-prefix
namespace case_3 {
struct B{S a;~B()noexcept{mark(10000);}};struct E:B{S b;E():b(2){} };
void probe(){E original;E copy(original);check(copy.a.value==7 && copy.b.value==2);}
}

// 200-synthesized-copy-defaulted-construction-prefix
namespace case_4 {
struct E{S a,b;E()=default;E(const E&)=default;E(E&&)=default;};
void probe(){E original;E copy(original);check(copy.a.value==7 && copy.b.value==7);}
}

// 200-synthesized-copy-empty-base-prefix-construction-prefix
namespace case_5 {
struct B{~B()noexcept{mark(10000);}};struct E:B{S a,b;};
void probe(){E original;E copy(original);check(copy.a.value==7 && copy.b.value==7);}
}

// 200-synthesized-copy-flat-construction-prefix
namespace case_6 {
struct E{S a,b;};
void probe(){S x(1); E original={x,x}; E copy(original);check(copy.a.value==1 && copy.b.value==1);}
}

// 200-synthesized-copy-member-custom-destructor-construction-prefix
namespace case_7 {
struct I{S a;~I()noexcept{mark(10000);}};struct E{I i;S b;};
void probe(){E original;E copy(original);check(copy.i.a.value==7 && copy.b.value==7);}
}

// 200-synthesized-copy-trivial-copy-array-prefix-construction-prefix
namespace case_8 {
struct T{int value;T():value(17){}~T()noexcept{mark(10000);}};struct E{T t[12];S a,b;};
void probe(){E original;E copy(original);for(int i=0;i<12;++i)check(copy.t[i].value==17);check(copy.a.value==7 && copy.b.value==7);}
}

// 200-synthesized-copy-trivial-copy-base-prefix-construction-prefix
namespace case_9 {
struct B{int value;B():value(17){}~B()noexcept{mark(10000);}};struct E:B{S a,b;};
void probe(){E original;E copy(original);check(copy.value==17 && copy.a.value==7 && copy.b.value==7);}
}

// 200-synthesized-copy-trivial-copy-member-prefix-construction-prefix
namespace case_10 {
struct T{int value;T():value(17){}~T()noexcept{mark(10000);}};struct E{T t;S a,b;};
void probe(){E original;E copy(original);check(copy.t.value==17 && copy.a.value==7 && copy.b.value==7);}
}

// 200-synthesized-copy-trivial-copy-union-prefix-construction-prefix
namespace case_11 {
union U{int value;U():value(17){}~U()noexcept{mark(10000);}};struct E{U u;S a,b;};
void probe(){E original;E copy(original);check(copy.u.value==17 && copy.a.value==7 && copy.b.value==7);}
}

// 200-synthesized-move-array-12-construction-prefix
namespace case_12 {
struct E{S a[12];S b;};
void probe(){E original;E copy(static_cast<E&&>(original));for(int i=0;i<12;++i)check(copy.a[i].value==7);check(copy.b.value==7);}
}

// 200-synthesized-move-flat-construction-prefix
namespace case_13 {
struct E{S a,b;};
void probe(){S x(1); E original={x,x}; E copy(static_cast<E&&>(original));check(copy.a.value==1 && copy.b.value==1);}
}

int run(void (*probe)(), int mark_scale) {
  bad = 0;
  int bound = 0;
  for (limit = 0; limit <= bound + 1; ++limit) {
    progress = live = count = marks = 0;
    int caught = 0;
    try { probe(); } catch (int n) { caught = n; }
    if (limit == 0) bound = progress;
    if (live || caught != (limit > 0 && limit <= bound ? limit : 0)) ++bad;
    if (mark_scale > 0 && marks != mark_scale * (limit == 1 || limit == 2 ? 1 : 2)) ++bad;
    if (mark_scale == -1 && marks != (limit == 1 ? 0 : (limit == 2 || limit == 3 ? 1 : 2))) ++bad;
  }
  return bad;
}
int main() {
  if (run(case_0::probe, 0)) return 1;
  if (run(case_1::probe, 0)) return 2;
  if (run(case_2::probe, 0)) return 3;
  if (run(case_3::probe, -1)) return 4;
  if (run(case_4::probe, 0)) return 5;
  if (run(case_5::probe, 1)) return 6;
  if (run(case_6::probe, 0)) return 7;
  if (run(case_7::probe, -1)) return 8;
  if (run(case_8::probe, 12)) return 9;
  if (run(case_9::probe, 1)) return 10;
  if (run(case_10::probe, 1)) return 11;
  if (run(case_11::probe, 1)) return 12;
  if (run(case_12::probe, 0)) return 13;
  if (run(case_13::probe, 0)) return 14;
  return 0;
}
