// lowir.md classes, result_classes: a direct object of at most 16 bytes states
// its eightbytes' classes exactly when some eightbyte is SSE or none: each
// eightbyte is INTEGER if a member's byte in it holds integer or pointer data,
// else SSE if one holds floating data, else none (psABI 3.2.3). The native
// back end passes an SSE eightbyte in the next XMM register (xmm0, xmm1 for a
// result) and an INTEGER one in the next general register.
struct Doubles { double a, b; };
struct DoubleLong { double d; long i; };
struct IntDouble { int i; double d; };
struct Floats { float a, b; };
struct ThreeFloats { float a, b, c; };
struct Ints { int a, b; };

double sum_doubles(Doubles p) { return p.a + p.b; }
double sum_mixed(DoubleLong p) { return p.d + p.i; }
double sum_reversed(IntDouble p) { return p.i + p.d; }
float sum_floats(Floats p) { return p.a + p.b; }
float third(ThreeFloats p) { return p.c; }
int sum_ints(Ints p) { return p.a + p.b; }
Doubles make_doubles(double x) { Doubles r = {x, 2 * x}; return r; }
DoubleLong make_mixed(double x) { DoubleLong r = {x, 3}; return r; }
Floats make_floats(float x) { Floats r = {x, x + 1}; return r; }

int main()
{
  Doubles d = make_doubles(1.5);
  DoubleLong m = make_mixed(2.5);
  IntDouble id = {1, 0.5};
  Floats f = make_floats(1.0f);
  ThreeFloats t = {1.0f, 2.0f, 3.0f};
  Ints n = {4, 5};
  return sum_doubles(d) == 4.5 && sum_mixed(m) == 5.5 && sum_reversed(id) == 1.5 &&
    sum_floats(f) == 3.0f && third(t) == 3.0f && sum_ints(n) == 9 ? 0 : 1;
}
