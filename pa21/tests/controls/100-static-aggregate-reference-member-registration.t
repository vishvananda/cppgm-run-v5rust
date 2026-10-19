// expect exit 0
// Reference initialization retries after partial aggregate construction;
// successful static objects register their reference-member temporaries first.
int live, attempts, order;
struct Verify {
  ~Verify() { if (live || order!=3221) __builtin_abort(); }
} verify;
struct S {
  int value;
  S(int n) : value(n) { if (n==2 && ++attempts==1) throw n; ++live; }
  ~S() noexcept { --live; order=order*10+value; }
};
struct Pair { S first; S second; };
void retry() { static const Pair& pair=Pair{1,2}; (void)pair; }
struct Holder { const S& reference; S value; };
void hold() { static Holder holder={S(2),3}; (void)holder; }
int main() {
  try { retry(); return 1; }
  catch (int n) { if (n!=2 || live) return 2; }
  order=0;
  retry();
  retry();
  // The pair's two members end after Holder and its referent.
  hold();
  return live!=4;
}
